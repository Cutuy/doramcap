use super::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use zbus::{MessageStream, OwnedObjectPath, OwnedVariant};

const SCOPE: &str = "helper-probe.scope";
const SLICE: &str = "helper-probe.slice";
const DESCRIPTION: &str = "owned helper probe";
const ACCESS_DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";
const UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

async fn disappearance(
    stage: &'static str,
    failure: &'static str,
) -> (Result<()>, Result<()>, Vec<String>) {
    let mut bus = tokio::process::Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut address = String::new();
    BufReader::new(bus.stdout.take().unwrap())
        .read_line(&mut address)
        .await
        .unwrap();
    let server = zbus::connection::Builder::address(address.trim())
        .name("org.freedesktop.systemd1")
        .build()
        .await
        .unwrap();
    let mut messages = MessageStream::from(&server);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let observed = calls.clone();
    let group = format!("/doramcap-scope-disappearance-{}", std::process::id());
    let path = host_path(&group).unwrap();
    assert!(!path.exists());
    let replies = tokio::spawn(async move {
        let mut missing = false;
        while let Some(message) = messages.next().await {
            let message = message.unwrap();
            let header = message.header();
            if header.message_type() != zbus::message::Type::MethodCall {
                continue;
            }
            let member = header.member().unwrap().as_str();
            let property = if member == "Get" {
                Some(message.body().deserialize::<(String, String)>().unwrap().1)
            } else {
                None
            };
            let call = property
                .as_ref()
                .map_or_else(|| member.to_owned(), |p| format!("Get:{p}"));
            observed.lock().unwrap().push(call.clone());
            if (call == stage || missing) && member != "ListUnitsByPatterns" {
                if failure != ACCESS_DENIED {
                    missing = true;
                }
                let error = if member == "Get" {
                    failure
                } else {
                    NO_SUCH_UNIT
                };
                server
                    .reply_error(&header, error, &"fixture disappearance")
                    .await
                    .unwrap();
                continue;
            }
            match member {
                "GetUnit" => server
                    .reply(
                        &header,
                        &OwnedObjectPath::try_from("/org/freedesktop/systemd1/unit/probe").unwrap(),
                    )
                    .await
                    .unwrap(),
                "Get" => {
                    let value = match property.unwrap().as_str() {
                        "Slice" => SLICE.to_owned(),
                        "Description" => DESCRIPTION.to_owned(),
                        "ControlGroup" => group.clone(),
                        other => panic!("unexpected property {other}"),
                    };
                    server
                        .reply(&header, &OwnedVariant::from(zbus::Str::from(value)))
                        .await
                        .unwrap();
                }
                "KillUnit" => server.reply(&header, &()).await.unwrap(),
                "StopUnit" => {
                    let job = OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/1").unwrap();
                    server.reply(&header, &job).await.unwrap();
                    server
                        .emit_signal(
                            None::<&str>,
                            "/org/freedesktop/systemd1",
                            "org.freedesktop.systemd1.Manager",
                            "JobRemoved",
                            &(1u32, job, SCOPE, "done"),
                        )
                        .await
                        .unwrap();
                }
                "ListUnitsByPatterns" => {
                    type ListedUnit = (
                        String,
                        String,
                        String,
                        String,
                        String,
                        String,
                        OwnedObjectPath,
                        u32,
                        String,
                        OwnedObjectPath,
                    );
                    server
                        .reply(&header, &Vec::<ListedUnit>::new())
                        .await
                        .unwrap();
                }
                _ => panic!("unexpected request {member}"),
            }
        }
    });
    let client = zbus::connection::Builder::address(address.trim())
        .build()
        .await
        .unwrap();
    let manager = ManagerProxy::new(&client).await.unwrap();
    let results = tokio::time::timeout(DEADLINE, async {
        (
            stop(&manager, SCOPE, SLICE, &path, DESCRIPTION).await,
            stop(&manager, SCOPE, SLICE, &path, DESCRIPTION).await,
        )
    })
    .await;
    replies.abort();
    let reply_result = replies.await;
    bus.kill().await.unwrap();
    assert!(reply_result.is_ok() || reply_result.unwrap_err().is_cancelled());
    let results = results.expect("scope cleanup did not finish");
    let calls = calls.lock().unwrap().clone();
    (results.0, results.1, calls)
}

#[tokio::test]
async fn absence_at_lookup_each_property_kill_and_stop_is_repeatable() {
    for stage in [
        "GetUnit",
        "Get:Slice",
        "Get:Description",
        "Get:ControlGroup",
        "KillUnit",
        "StopUnit",
    ] {
        // Arrange / Act
        let (first, repeated, calls) = disappearance(stage, UNKNOWN_OBJECT).await;
        // Assert
        assert!(first.is_ok(), "{stage}: {first:?}; {calls:?}");
        assert!(repeated.is_ok(), "{stage}: {repeated:?}; {calls:?}");
        if stage.starts_with("Get") {
            assert!(!calls.iter().any(|c| c == "KillUnit"));
        }
    }
}
#[tokio::test]
async fn denied_properties_preserve_the_unit_before_signaling() {
    for stage in ["Get:Slice", "Get:Description", "Get:ControlGroup"] {
        // Arrange / Act
        let (first, repeated, calls) = disappearance(stage, ACCESS_DENIED).await;
        // Assert
        assert!(first.unwrap_err().contains(ACCESS_DENIED));
        assert!(repeated.unwrap_err().contains(ACCESS_DENIED));
        assert!(!calls.iter().any(|c| c == "KillUnit"));
    }
}

#[tokio::test]
async fn a_failed_owned_scope_is_reset_after_its_completed_stop_job() {
    // Arrange / Act
    let (first, repeated, calls) = disappearance("ResetFailedUnit", NO_SUCH_UNIT).await;
    // Assert: reset accepts disappearance, unloads, and repetition accepts absence.
    first.unwrap();
    repeated.unwrap();
    let stop = calls.iter().position(|c| c == "StopUnit").unwrap();
    let reset = calls.iter().position(|c| c == "ResetFailedUnit").unwrap();
    assert!(stop < reset);
    assert!(calls.iter().any(|c| c == "ListUnitsByPatterns"));
}
