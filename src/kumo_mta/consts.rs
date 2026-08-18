pub const KUMOD_EXECUTABLE: &str = "/opt/kumomta/sbin/kumod";
pub const KCLI_EXECUTABLE: &str = "/opt/kumomta/sbin/kcli";

/// kumod drops its privileges to this user - the process itself is started by our
/// service, which is running as root inside the container.
pub const KUMOD_USER: &str = "kumod";

pub const POLICY_DIR: &str = "/opt/kumomta/etc/policy";
pub const POLICY_FILE: &str = "/opt/kumomta/etc/policy/init.lua";
pub const DKIM_KEYS_DIR: &str = "/opt/kumomta/etc/dkim";

/// The spool has to survive the restart of the container - otherwise the mail which
/// is accepted but not delivered yet is lost.
pub const SPOOL_DATA_DIR: &str = "/var/spool/kumomta/data";
pub const SPOOL_META_DIR: &str = "/var/spool/kumomta/meta";
pub const LOG_DIR: &str = "/var/log/kumomta";

/// Endpoint of the kumod instance which is running inside the same container.
pub const LOCAL_SMTP_HOST: &str = "127.0.0.1";
pub const LOCAL_SMTP_PORT: u16 = 25;

/// kumod http api - it is used to read the state of the queues. Our own http server
/// is listening on the port 8000, that is why kumod gets a different one.
pub const KUMO_HTTP_LISTENER: &str = "127.0.0.1:8009";
