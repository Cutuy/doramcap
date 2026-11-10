//! Typed Dora control owned by one experiment through startup and verified cleanup.
//! Transport frame/message limits use tungstenite defaults; excess input fails explicitly.
use dora_message::{
    DataflowId,
    cli_to_coordinator::ControlRequest,
    coordinator_to_cli::{ControlRequestReply, DataflowList, DataflowStatus},
    id::NodeId,
    ws_protocol::{WsMessage, WsRequest},
};
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpStream, time::Instant};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest, http::header::AUTHORIZATION},
};

pub use dora_message as protocol;
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
const CONTROL_PATH: &str = "/api/control";
const CONTROL_METHOD: &str = "control";
// Approved startup sampling exception; replace through the tracked upstream-readiness TODO.
const DAEMON_REGISTRATION_CHECK_INTERVAL: Duration = Duration::from_millis(100);

/// A failed control operation; a sent mutation may have taken effect remotely.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Dora control deadline expired")]
    Deadline,
    #[error("Dora control transport: {0}")]
    Transport(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Dora control JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Dora control protocol: {0}")]
    Protocol(String),
    #[error("Dora coordinator: {0}")]
    Remote(String),
    #[error("Dora graph identity: {0}")]
    Identity(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Attempted graph identity, retained even if its Start reply is lost.
#[derive(Clone, Debug)]
pub struct Graph {
    /// Exact experiment name used for safe registration reconciliation.
    pub name: String,
    /// None means the attempted Start has no confirmed UUID yet.
    pub uuid: Option<DataflowId>,
}

/// One experiment's reusable connection and graph identity; serialize access externally.
/// Cancellation drops the in-flight socket and retains graph state for cleanup.
/// Endpoint ownership and native process teardown belong to the caller.
pub struct DoraCoordinatorClient {
    endpoint: SocketAddr,
    bearer_token: Option<String>,
    connection: Option<Socket>,
    graph: Option<Graph>,
}
impl DoraCoordinatorClient {
    /// Use an explicit endpoint and optional bearer token; performs no I/O or ambient discovery.
    pub fn new(endpoint: SocketAddr, bearer_token: Option<String>) -> Self {
        Self {
            endpoint,
            bearer_token,
            connection: None,
            graph: None,
        }
    }

    /// Return the retained attempted or known graph identity.
    pub fn graph(&self) -> Option<&Graph> {
        self.graph.as_ref()
    }

    /// Drop the transport immediately while retaining graph ownership for cleanup.
    pub fn disconnect(&mut self) {
        self.connection = None;
    }

    /// Connect and negotiate Hello before deadline; reuses an established connection.
    pub async fn connect(&mut self, deadline: Instant) -> Result<()> {
        if self.connection.is_some() {
            return Ok(());
        }
        check_deadline(deadline)?;
        let mut request = format!("ws://{}{CONTROL_PATH}", self.endpoint).into_client_request()?;
        if let Some(token) = &self.bearer_token {
            let header = format!("Bearer {token}")
                .parse()
                .map_err(|_| Error::Protocol("invalid explicit authentication token".into()))?;
            request.headers_mut().insert(AUTHORIZATION, header);
        }
        let handshake = async {
            let (mut socket, _) = tokio_tungstenite::connect_async(request).await?;
            match exchange(&mut socket, ControlRequest::hello()).await? {
                ControlRequestReply::HelloOk { dora_version } => {
                    dora_message::cli_to_coordinator::check_cli_version(&dora_version)
                        .map_err(Error::Protocol)?;
                }
                other => return Err(unexpected("Hello", other)),
            }
            Ok(socket)
        };
        self.connection = Some(bounded(deadline, handshake).await?);
        Ok(())
    }

    /// Send one typed request before deadline; never retries a sent operation.
    /// Cancellation, transport errors and deadline expiry discard the socket.
    async fn request(
        &mut self,
        request: ControlRequest,
        deadline: Instant,
    ) -> Result<ControlRequestReply> {
        check_deadline(deadline)?;
        self.connect(deadline).await?;
        check_deadline(deadline)?;
        // A cancelled future owns the socket, while the experiment keeps this client.
        let mut socket = self.connection.take().expect("connected socket");
        let reply = bounded(deadline, exchange(&mut socket, request)).await?;
        self.connection = Some(socket);
        Ok(reply)
    }

    /// Return whether a daemon is registered; the caller owns approved readiness sampling.
    pub async fn daemon_connected(&mut self, deadline: Instant) -> Result<bool> {
        match self
            .request(ControlRequest::DaemonConnected, deadline)
            .await?
        {
            ControlRequestReply::DaemonConnected(connected) => Ok(connected),
            other => Err(self.reject("DaemonConnected", other)),
        }
    }

    /// Await daemon registration with approved startup sampling, bounded by deadline.
    /// TODO: replace timed checks with upstream notification; see the implementation plan.
    pub async fn wait_for_daemon(&mut self, deadline: Instant) -> Result<()> {
        let mut checks = tokio::time::interval(DAEMON_REGISTRATION_CHECK_INTERVAL);
        checks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            bounded(deadline, async {
                checks.tick().await;
                Ok(())
            })
            .await?;
            match self.daemon_connected(deadline).await {
                Ok(true) => return Ok(()),
                Ok(false) => (),
                Err(Error::Transport(tokio_tungstenite::tungstenite::Error::Io(error)))
                    if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
                Err(error) => return Err(error),
            }
        }
    }

    /// List typed graph entries before deadline.
    pub async fn list(&mut self, deadline: Instant) -> Result<DataflowList> {
        match self.request(ControlRequest::List, deadline).await? {
            ControlRequestReply::DataflowList(list) => Ok(list),
            other => Err(self.reject("List", other)),
        }
    }

    /// Start a named graph once and retain its UUID before awaiting spawn completion.
    /// Requires a Start request with explicit unique name and caller-prepared session/paths.
    pub async fn start_graph(
        &mut self,
        start: ControlRequest,
        deadline: Instant,
    ) -> Result<DataflowId> {
        let ControlRequest::Start {
            name: Some(name), ..
        } = &start
        else {
            return Err(Error::Identity(
                "Start requires an explicit run name".into(),
            ));
        };
        if name.is_empty() {
            return Err(Error::Identity("Start requires a nonempty run name".into()));
        }
        if self.graph.is_some() {
            return Err(Error::Identity("graph start already attempted".into()));
        }
        check_deadline(deadline)?;
        // Persist intent before the first cancellable connect or send.
        self.graph = Some(Graph {
            name: name.clone(),
            uuid: None,
        });
        let uuid = match self.request(start, deadline).await? {
            ControlRequestReply::DataflowStartTriggered { uuid } => uuid,
            other => return Err(self.reject("Start", other)),
        };
        self.graph.as_mut().expect("recorded start").uuid = Some(uuid);
        match self
            .request(ControlRequest::WaitForSpawn { dataflow_id: uuid }, deadline)
            .await?
        {
            ControlRequestReply::DataflowSpawned { uuid: spawned } if spawned == uuid => Ok(uuid),
            other => Err(self.reject("WaitForSpawn", other)),
        }
    }

    /// Verify the retained graph UUID/name still identify one running dataflow.
    pub async fn inspect_graph(&mut self, deadline: Instant) -> Result<()> {
        let Some(graph) = self.graph.clone() else {
            return Ok(());
        };
        let uuid = graph
            .uuid
            .ok_or_else(|| Error::Identity("registration outcome unknown".into()))?;
        let list = self.list(deadline).await?;
        let mut entries = list
            .0
            .iter()
            .filter(|entry| entry.id.name.as_deref() == Some(&graph.name));
        match (entries.next(), entries.next()) {
            (Some(entry), None)
                if entry.id.uuid == uuid && entry.status == DataflowStatus::Running =>
            {
                Ok(())
            }
            _ => Err(Error::Identity(
                "owned graph stopped or is ambiguous".into(),
            )),
        }
    }

    /// Recover an uncertain Start only from one exact run-name match; never starts a graph.
    pub async fn reconcile_graph(&mut self, deadline: Instant) -> Result<Option<DataflowId>> {
        let Some(graph) = self.graph.clone() else {
            return Ok(None);
        };
        if let Some(uuid) = graph.uuid {
            return Ok(Some(uuid));
        }
        let list = self.list(deadline).await?;
        let mut entries = list
            .0
            .iter()
            .filter(|entry| entry.id.name.as_deref() == Some(&graph.name));
        let uuid = match (entries.next(), entries.next()) {
            (Some(entry), None) => entry.id.uuid,
            _ => {
                return Err(Error::Identity(
                    "start outcome unresolved; no unique owned graph".into(),
                ));
            }
        };
        self.graph.as_mut().expect("retained attempt").uuid = Some(uuid);
        Ok(Some(uuid))
    }

    /// Stop one system node of the owned graph; preserves graph identity on every outcome.
    pub async fn stop_node(
        &mut self,
        node_id: NodeId,
        grace: Duration,
        deadline: Instant,
    ) -> Result<()> {
        let Some(uuid) = self.reconcile_graph(deadline).await? else {
            return Ok(());
        };
        let request = ControlRequest::StopNode {
            dataflow_id: uuid,
            node_id: node_id.clone(),
            grace_duration: Some(grace),
        };
        match self.request(request, deadline).await? {
            ControlRequestReply::NodeStopped {
                dataflow_id,
                node_id: stopped,
            } if dataflow_id == uuid && stopped == node_id => Ok(()),
            other => Err(self.reject("StopNode", other)),
        }
    }

    /// Stop the owned graph before Destroy; return errors so native teardown can continue.
    /// An unresolved identity or unconfirmed stop prevents protocol-level destruction.
    pub async fn shutdown(&mut self, grace: Duration, deadline: Instant) -> Result<()> {
        let result = self.shutdown_inner(grace, deadline).await;
        self.disconnect();
        result
    }

    async fn shutdown_inner(&mut self, grace: Duration, deadline: Instant) -> Result<()> {
        if let Some(uuid) = self.reconcile_graph(deadline).await? {
            let request = ControlRequest::Stop {
                dataflow_uuid: uuid,
                grace_duration: Some(grace),
                force: true,
            };
            match self.request(request, deadline).await? {
                ControlRequestReply::DataflowStopped {
                    uuid: stopped,
                    result,
                } if stopped == uuid => {
                    if !result.is_ok() {
                        return Err(Error::Remote(format!(
                            "dataflow stop failed: {:?}",
                            result.node_results
                        )));
                    }
                }
                other => return Err(self.reject("Stop", other)),
            }
        }
        match self.request(ControlRequest::Destroy, deadline).await? {
            ControlRequestReply::DestroyOk => Ok(()),
            other => Err(self.reject("Destroy", other)),
        }
    }

    fn reject(&mut self, operation: &str, reply: ControlRequestReply) -> Error {
        self.disconnect();
        unexpected(operation, reply)
    }
}

fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(Error::Deadline)
    } else {
        Ok(())
    }
}
async fn bounded<T>(
    deadline: Instant,
    operation: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::time::timeout_at(deadline, operation)
        .await
        .map_err(|_| Error::Deadline)?
}
fn unexpected(operation: &str, reply: ControlRequestReply) -> Error {
    Error::Protocol(format!("unexpected reply to {operation}: {reply:?}"))
}

async fn exchange(socket: &mut Socket, request: ControlRequest) -> Result<ControlRequestReply> {
    let id = DataflowId::new_v4();
    let request = WsRequest {
        id,
        method: CONTROL_METHOD.into(),
        params: serde_json::to_value(request)?,
    };
    socket
        .send(Message::Text(serde_json::to_string(&request)?.into()))
        .await?;
    while let Some(frame) = socket.next().await {
        match frame? {
            Message::Text(text) => {
                let WsMessage::Response(response) = serde_json::from_str(&text)? else {
                    return Err(Error::Protocol(
                        "unexpected control event or request".into(),
                    ));
                };
                if response.id != id {
                    return Err(Error::Protocol(
                        "response ID does not match pending request".into(),
                    ));
                }
                if let Some(error) = response.error {
                    return Err(Error::Remote(error));
                }
                let result = response
                    .result
                    .ok_or_else(|| Error::Protocol("response has no result".into()))?;
                let reply = serde_json::from_value(result)?;
                return match reply {
                    ControlRequestReply::Error(error) => Err(Error::Remote(error)),
                    other => Ok(other),
                };
            }
            Message::Ping(_) => socket.flush().await?,
            Message::Pong(_) => (),
            Message::Close(_) => {
                return Err(Error::Protocol("coordinator closed the connection".into()));
            }
            _ => return Err(Error::Protocol("unexpected binary control frame".into())),
        }
    }
    Err(Error::Protocol(
        "coordinator disconnected before replying".into(),
    ))
}
