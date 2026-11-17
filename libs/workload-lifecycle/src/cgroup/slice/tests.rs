use super::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use zbus::{MessageStream, OwnedObjectPath, OwnedVariant};

const SLICE: &str = "cleanup-probe.slice";
const MANAGER: &str = "org.freedesktop.systemd1";
const ACCESS_DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
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

/// Reply through a private bus so the real proxies observe disappearance after lookup.
async fn remove_with_reply(
    stop_error: &'static str,
    group: String,
) -> (Result<()>, Result<()>, Vec<String>) {
    let mut bus = tokio::process::Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("private D-Bus test requires dbus-daemon");
    let mut address = String::new();
    BufReader::new(bus.stdout.take().unwrap())
        .read_line(&mut address)
        .await
        .unwrap();
    let server = zbus::connection::Builder::address(address.trim())
        .name(MANAGER)
        .build()
        .await
        .unwrap();
    let mut messages = MessageStream::from(&server);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let observed = calls.clone();
    let replies = tokio::spawn(async move {
        let mut missing = false;
        while let Some(message) = messages.next().await {
            let message = message.unwrap();
            let header = message.header();
            if header.message_type() != zbus::message::Type::MethodCall {
                continue;
            }
            let member = header.member().unwrap().as_str();
            observed.lock().unwrap().push(member.to_owned());
            match member {
                "GetUnit" if missing => {
                    server
                        .reply_error(&header, NO_SUCH_UNIT, &"already absent")
                        .await
                        .unwrap();
                }
                "GetUnit" => {
                    let path =
                        OwnedObjectPath::try_from("/org/freedesktop/systemd1/unit/probe").unwrap();
                    server.reply(&header, &path).await.unwrap();
                }
                "Get" => {
                    let property = OwnedVariant::from(zbus::Str::from(group.clone()));
                    server.reply(&header, &property).await.unwrap();
                }
                "StopUnit" => {
                    missing = stop_error == NO_SUCH_UNIT;
                    server
                        .reply_error(&header, stop_error, &"stop response")
                        .await
                        .unwrap();
                }
                "ListUnitsByPatterns" => {
                    server
                        .reply(&header, &Vec::<ListedUnit>::new())
                        .await
                        .unwrap();
                }
                _ => panic!("unexpected systemd request: {member}"),
            }
        }
    });
    let client = zbus::connection::Builder::address(address.trim())
        .build()
        .await
        .unwrap();
    let manager = ManagerProxy::new(&client).await.unwrap();
    let results = tokio::time::timeout(DEADLINE, async {
        (stop(&manager, SLICE).await, stop(&manager, SLICE).await)
    })
    .await;
    replies.abort();
    let reply_result = replies.await;
    bus.kill().await.unwrap();
    assert!(reply_result.is_ok() || reply_result.unwrap_err().is_cancelled());
    let (first, repeated) = results.expect("slice cleanup did not finish");
    let observed = calls.lock().unwrap().clone();
    (first, repeated, observed)
}

#[tokio::test]
async fn disappearing_before_stop_is_removed_and_repeated_cleanup_succeeds() {
    // Arrange / Act: lookup and property inspection succeed, then StopUnit reports absence.
    let (first, repeated, calls) = remove_with_reply(NO_SUCH_UNIT, String::new()).await;
    // Assert: no nonexistent job is awaited, and the next cleanup accepts the missing unit.
    first.unwrap();
    repeated.unwrap();
    assert_eq!(calls.iter().filter(|call| *call == "StopUnit").count(), 1);
    assert!(calls.iter().any(|call| call == "ListUnitsByPatterns"));
}

#[tokio::test]
async fn other_stop_errors_remain_failures() {
    // Arrange / Act
    let (first, repeated, calls) = remove_with_reply(ACCESS_DENIED, String::new()).await;
    // Assert
    assert!(first.unwrap_err().contains(ACCESS_DENIED));
    assert!(repeated.unwrap_err().contains(ACCESS_DENIED));
    assert!(!calls.iter().any(|call| call == "ListUnitsByPatterns"));
}

#[tokio::test]
async fn a_live_group_is_preserved_before_any_stop_request() {
    // Arrange: the test's own cgroup contains this process and needs no privileged setup.
    let group = std::fs::read_to_string("/proc/self/cgroup")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap()
        .to_owned();
    // Act
    let (first, repeated, calls) = remove_with_reply(NO_SUCH_UNIT, group).await;
    // Assert
    assert!(first.unwrap_err().contains("live processes"));
    assert!(repeated.unwrap_err().contains("live processes"));
    assert!(!calls.iter().any(|call| call == "StopUnit"));
}
