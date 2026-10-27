//! Typed pbtxt payloads with ordered includes and reproducible deployments.
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/pbtxt_config.v1.rs"));
}
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}
pub type Result<T> = std::result::Result<T, Error>;
fn io(context: String, source: std::io::Error) -> Error {
    Error::Io { context, source }
}

/// Domain adapter. Validate every partial input; normalize completed payloads.
/// `overlay` returns assigned field names, including explicitly supplied defaults.
/// Normalized protobuf encoding must be deterministic for reproducible digests.
pub trait Payload: Message + Default + Clone {
    const NAME: &'static str;
    const DESCRIPTOR: &'static [u8];
    fn validate_partial(&self) -> Result<()>;
    fn overlay(&self, target: &mut Self) -> Vec<&'static str>;
    fn normalize(&self) -> Result<Self>;
}
#[derive(Clone, Debug)]
pub struct App<P> {
    pub id: String,
    pub config: Option<P>,
}
#[derive(Clone, Debug)]
pub struct Fragment<P> {
    pub include: Vec<String>,
    pub app: Vec<App<P>>,
}
#[derive(Clone, Debug)]
pub struct Override {
    pub app: String,
    pub field: &'static str,
    pub source: PathBuf,
}
#[derive(Debug)]
pub struct Resolved<P> {
    deployment: proto::Deployment,
    pub provenance: Vec<Override>,
    pub sources: BTreeMap<PathBuf, Vec<u8>>,
    marker: std::marker::PhantomData<P>,
}
impl<P: Payload> Resolved<P> {
    pub fn deployment(&self) -> &proto::Deployment {
        &self.deployment
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.deployment.encode_to_vec()
    }
    pub fn digest(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_bytes()).into()
    }
    pub fn config(&self, id: &str) -> Result<P> {
        let app = self
            .deployment
            .app
            .iter()
            .find(|app| app.id == id)
            .ok_or_else(|| Error::Invalid(format!("unknown app {id:?}")))?;
        unpack(app.config.as_ref().expect("resolved payload"))
    }
}
fn pack<P: Payload>(value: &P) -> prost_types::Any {
    prost_types::Any {
        type_url: format!("type.googleapis.com/{}", P::NAME),
        value: value.encode_to_vec(),
    }
}
fn unpack<P: Payload>(value: &prost_types::Any) -> Result<P> {
    if value.type_url.rsplit('/').next() != Some(P::NAME) {
        return Err(Error::Invalid(format!(
            "expected {}, got {}",
            P::NAME,
            value.type_url
        )));
    }
    P::decode(value.value.as_slice()).map_err(|e| Error::Invalid(e.to_string()))
}
pub fn descriptor_pool<P: Payload>() -> Result<DescriptorPool> {
    let mut pool = DescriptorPool::decode(DESCRIPTOR).map_err(|e| Error::Invalid(e.to_string()))?;
    pool.decode_file_descriptor_set(P::DESCRIPTOR)
        .map_err(|e| Error::Invalid(e.to_string()))?;
    Ok(pool)
}
pub fn parse_fragment<P: Payload>(text: &str) -> Result<Fragment<P>> {
    let pool = descriptor_pool::<P>()?;
    let desc = pool
        .get_message_by_name("pbtxt_config.v1.Fragment")
        .expect("compiled descriptor");
    let raw: proto::Fragment = DynamicMessage::parse_text_format(desc, text)
        .map_err(|e| Error::Invalid(e.to_string()))?
        .transcode_to()
        .map_err(|e| Error::Invalid(e.to_string()))?;
    Ok(Fragment {
        include: raw.include,
        app: raw
            .app
            .into_iter()
            .map(|app| {
                Ok(App {
                    id: app.id,
                    config: app.config.as_ref().map(unpack).transpose()?,
                })
            })
            .collect::<Result<_>>()?,
    })
}
impl<P: Payload> Fragment<P> {
    pub fn to_proto(&self) -> proto::Fragment {
        proto::Fragment {
            include: self.include.clone(),
            app: self
                .app
                .iter()
                .map(|app| proto::App {
                    id: app.id.clone(),
                    config: app.config.as_ref().map(pack),
                })
                .collect(),
        }
    }
}
/// `registry` is authoritative; every registered app receives a normalized payload.
/// Repeated includes reapply a cached source at their declared precedence position.
/// Loading rejects more than 32 active include levels, 256 total visits, 1 MiB per
/// file or 8 MiB across unique sources. Limit errors name the failing path and limit;
/// oversized-file diagnostics report a lower bound from the bounded read. Cycles,
/// invalid fragments, unknown app IDs and payload validation failures also return
/// errors. Resolution performs no writes and returns no partial deployment.
pub fn resolve<P: Payload>(path: &Path, registry: &[&str]) -> Result<Resolved<P>> {
    let mut loader = Loader::<P> {
        payloads: BTreeMap::new(),
        stack: Vec::new(),
        sources: BTreeMap::new(),
        provenance: Vec::new(),
        visits: 0,
        bytes: 0,
    };
    for id in registry {
        if id.is_empty()
            || loader
                .payloads
                .insert((*id).to_owned(), P::default())
                .is_some()
        {
            return Err(Error::Invalid(format!(
                "empty or duplicate registry ID {id:?}"
            )));
        }
    }
    loader.visit(path)?;
    let app = loader
        .payloads
        .into_iter()
        .map(|(id, policy)| {
            let policy = policy
                .normalize()
                .map_err(|e| Error::Invalid(format!("app {id}: {e}")))?;
            Ok(proto::App {
                id,
                config: Some(pack(&policy)),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Resolved {
        deployment: proto::Deployment { version: 1, app },
        provenance: loader.provenance,
        sources: loader.sources,
        marker: std::marker::PhantomData,
    })
}
struct Loader<P: Payload> {
    payloads: BTreeMap<String, P>,
    stack: Vec<PathBuf>,
    sources: BTreeMap<PathBuf, Vec<u8>>,
    provenance: Vec<Override>,
    visits: usize,
    bytes: usize,
}
impl<P: Payload> Loader<P> {
    fn visit(&mut self, path: &Path) -> Result<()> {
        self.visits += 1;
        if self.stack.len() >= 32 {
            return Err(Error::Invalid(format!(
                "{}: include depth limit exceeded (limit: 32, observed: {})",
                path.display(),
                self.stack.len() + 1
            )));
        }
        if self.visits > 256 {
            return Err(Error::Invalid(format!(
                "{}: include visit limit exceeded (limit: 256, observed: {})",
                path.display(),
                self.visits
            )));
        }
        let path = fs::canonicalize(path).map_err(|e| io(path.display().to_string(), e))?;
        if self.stack.contains(&path) {
            return Err(Error::Invalid(format!(
                "include cycle: {:?} -> {}",
                self.stack,
                path.display()
            )));
        }
        if !self.sources.contains_key(&path) {
            let mut data = Vec::new();
            fs::File::open(&path)
                .map_err(|e| io(path.display().to_string(), e))?
                .take(1_048_577)
                .read_to_end(&mut data)
                .map_err(|e| io(path.display().to_string(), e))?;
            self.bytes += data.len();
            if data.len() > 1_048_576 {
                return Err(Error::Invalid(format!(
                    "{}: configuration byte limit exceeded for file (limit: 1048576, observed at least: {})",
                    path.display(),
                    data.len()
                )));
            }
            if self.bytes > 8_388_608 {
                return Err(Error::Invalid(format!(
                    "{}: configuration byte limit exceeded for all sources (limit: 8388608, observed: {})",
                    path.display(),
                    self.bytes
                )));
            }
            self.sources.insert(path.clone(), data);
        }
        let text = std::str::from_utf8(&self.sources[&path])
            .map_err(|e| Error::Invalid(format!("{}: {e}", path.display())))?;
        let fragment = parse_fragment::<P>(text)
            .map_err(|e| Error::Invalid(format!("{}: {e}", path.display())))?;
        // Check every fragment even when a later overlay would hide its error.
        let mut seen = BTreeSet::new();
        for app in &fragment.app {
            if !seen.insert(&app.id) || !self.payloads.contains_key(&app.id) {
                return Err(Error::Invalid(format!(
                    "{}: duplicate or unknown app {:?}",
                    path.display(),
                    app.id
                )));
            }
            if let Some(policy) = &app.config {
                policy.validate_partial().map_err(|e| {
                    Error::Invalid(format!("{} app {}: {e}", path.display(), app.id))
                })?;
            }
        }
        self.stack.push(path.clone());
        for include in fragment.include {
            if include.is_empty() {
                return Err(Error::Invalid(format!("{}: empty include", path.display())));
            }
            self.visit(&path.parent().expect("canonical file parent").join(include))?;
        }
        for app in fragment.app {
            if let Some(policy) = app.config {
                let target = self.payloads.get_mut(&app.id).expect("validated registry");
                for field in policy.overlay(target) {
                    self.provenance.push(Override {
                        app: app.id.clone(),
                        field,
                        source: path.clone(),
                    });
                }
            }
        }
        self.stack.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    // Reuse a known descriptor as a payload to exercise the resolver without a
    // domain crate. Its ID behaves like a required, replaceable string setting.
    impl Payload for proto::App {
        const NAME: &'static str = "pbtxt_config.v1.App";
        const DESCRIPTOR: &'static [u8] = DESCRIPTOR;
        fn validate_partial(&self) -> Result<()> {
            if self.id == "invalid" {
                return Err(Error::Invalid("invalid ID".into()));
            }
            Ok(())
        }
        fn overlay(&self, target: &mut Self) -> Vec<&'static str> {
            if self.id.is_empty() {
                return vec![];
            }
            target.id.clone_from(&self.id);
            vec!["id"]
        }
        fn normalize(&self) -> Result<Self> {
            if self.id.is_empty() {
                return Err(Error::Invalid("missing ID".into()));
            }
            Ok(self.clone())
        }
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "pbtxt-config-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn put(&self, name: &str, text: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, text).unwrap();
            path
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn entry(id: &str, value: &str) -> String {
        format!(
            "app {{ id: '{id}' config {{ [type.googleapis.com/pbtxt_config.v1.App] {{ id: '{value}' }} }} }}"
        )
    }
    #[test]
    fn independent_payload_composition_and_fingerprint() {
        // Arrange
        let tmp = Temp::new();
        tmp.put("base", &entry("a", "base"));
        tmp.put("override", &entry("a", "lab"));
        let path = tmp.put("main", "include:'base' include:'override' include:'base'");
        // Act
        let resolved = resolve::<proto::App>(&path, &["a"]).unwrap();

        // Assert
        assert_eq!(resolved.config("a").unwrap().id, "base");
        assert_eq!(resolved.provenance.len(), 3);
        assert_eq!(resolved.sources.len(), 3);
        assert!(resolved.config("absent").is_err());
        // Arrange
        let single = tmp.put("single", &entry("a", "base"));

        // Act / Assert: equivalent resolved payloads have the same fingerprint.
        assert_eq!(
            resolved.digest(),
            resolve::<proto::App>(&single, &["a"]).unwrap().digest()
        );
        let bytes = resolved.canonical_bytes();
        let wire = proto::Deployment::decode(bytes.as_slice()).unwrap();
        assert_eq!(
            wire.app[0].config.as_ref().unwrap().type_url,
            "type.googleapis.com/pbtxt_config.v1.App"
        );
    }
    #[test]
    fn rejects_wrong_types_and_hidden_invalid_inputs() {
        // Arrange
        let tmp = Temp::new();
        for bad in [
            entry("a", "invalid"),
            entry("unknown", "ok"),
            format!("{} {}", entry("a", "ok"), entry("a", "ok")),
            "app { id:'a' config { [type.googleapis.com/pbtxt_config.v1.Deployment] {} } }".into(),
        ] {
            tmp.put("bad", &bad);
            let path = tmp.put("main", &format!("include:'bad' {}", entry("a", "good")));
            // Act
            let error = resolve::<proto::App>(&path, &["a"]).unwrap_err();

            // Assert
            assert!(error.to_string().contains("bad"));
        }
    }
    #[test]
    fn cycles_and_registry_validation() {
        // Arrange
        let tmp = Temp::new();
        let path = tmp.put("a", "include:'b'");
        tmp.put("b", "include:'a'");

        // Act
        let error = resolve::<proto::App>(&path, &[]).unwrap_err();

        // Assert
        assert!(error.to_string().contains("cycle"));

        // Arrange
        tmp.put("a", "include:''");

        // Act / Assert: empty includes are invalid before deployment normalization.
        assert!(resolve::<proto::App>(&path, &[]).is_err());

        // Arrange
        tmp.put("a", "");

        // Act / Assert: missing payloads and invalid registry IDs are rejected.
        assert!(resolve::<proto::App>(&path, &["a"]).is_err());
        assert!(resolve::<proto::App>(&path, &["a", "a"]).is_err());
        assert!(resolve::<proto::App>(&path, &[""]).is_err());
    }

    #[test]
    fn include_depth_boundary_reports_path_and_limit() {
        // Arrange: 32 active levels are supported, counting the root file.
        let tmp = Temp::new();
        for i in 0..31 {
            tmp.put(&format!("level{i}"), &format!("include:'level{}'", i + 1));
        }
        tmp.put("level31", "");
        let root = tmp.0.join("level0");

        // Act
        let result = resolve::<proto::App>(&root, &[]);

        // Assert
        assert!(result.is_ok());

        // Arrange: adding one more include crosses the depth boundary.
        tmp.put("level31", "include:'level32'");
        let offending = tmp.put("level32", "");

        // Act
        let error = resolve::<proto::App>(&root, &[]).unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            format!(
                "{}: include depth limit exceeded (limit: 32, observed: 33)",
                offending.display()
            )
        );
    }

    #[test]
    fn include_visit_boundary_counts_cached_reapplications() {
        // Arrange: the root plus 255 cached includes uses all 256 visits.
        let tmp = Temp::new();
        let included = tmp.put("base", "");
        let root = tmp.put("main", &"include:'base'\n".repeat(255));

        // Act
        let result = resolve::<proto::App>(&root, &[]).unwrap();

        // Assert: cached source bytes do not erase include visits.
        assert_eq!(result.sources.len(), 2);

        // Arrange
        tmp.put("main", &"include:'base'\n".repeat(256));

        // Act
        let error = resolve::<proto::App>(&root, &[]).unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            format!(
                "{}: include visit limit exceeded (limit: 256, observed: 257)",
                included.display()
            )
        );
    }

    #[test]
    fn file_byte_boundary_reports_a_bounded_read_lower_bound() {
        // Arrange: whitespace is a valid empty fragment at exactly 1 MiB.
        let tmp = Temp::new();
        let path = tmp.put("main", &" ".repeat(1_048_576));

        // Act
        let result = resolve::<proto::App>(&path, &[]);

        // Assert
        assert!(result.is_ok());

        // Arrange: the bounded reader observes only the first excess byte.
        tmp.put("main", &" ".repeat(1_048_580));

        // Act
        let error = resolve::<proto::App>(&path, &[]).unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            format!(
                "{}: configuration byte limit exceeded for file (limit: 1048576, observed at least: 1048577)",
                path.display()
            )
        );
    }

    #[test]
    fn aggregate_byte_boundary_identifies_the_last_source() {
        // Arrange: the root and eight unique fragments total exactly 8 MiB.
        let tmp = Temp::new();
        let includes = (0..8)
            .map(|i| format!("include:'part{i}'\n"))
            .collect::<String>();
        let root = tmp.put("main", &includes);
        for i in 0..7 {
            tmp.put(&format!("part{i}"), &" ".repeat(1_048_576));
        }
        let last_size = 1_048_576 - includes.len();
        let last = tmp.put("part7", &" ".repeat(last_size));

        // Act
        let result = resolve::<proto::App>(&root, &[]);

        // Assert
        assert!(result.is_ok());

        // Arrange: the last file stays below 1 MiB but exceeds the total by one byte.
        tmp.put("part7", &" ".repeat(last_size + 1));

        // Act
        let error = resolve::<proto::App>(&root, &[]).unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            format!(
                "{}: configuration byte limit exceeded for all sources (limit: 8388608, observed: 8388609)",
                last.display()
            )
        );
    }
}
