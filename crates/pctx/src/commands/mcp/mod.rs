pub(crate) mod add;
pub(crate) mod dev;
pub(crate) mod init;
pub(crate) mod list;
pub(crate) mod remove;
pub(crate) mod start;

pub(crate) use add::AddCmd;

use pctx_config::Config;

pub(crate) fn has_stdio_upstreams(cfg: &Config) -> bool {
    cfg.servers.iter().any(|server| server.stdio().is_some())
}

#[cfg(test)]
mod tests {
    use super::has_stdio_upstreams;
    use pctx_config::{Config, server::ServerConfig};

    #[test]
    fn test_has_stdio_upstreams() {
        let mut cfg = Config::default();
        cfg.servers.push(ServerConfig::new_stdio(
            "local".to_string(),
            "echo".to_string(),
            vec!["hi".to_string()],
            Default::default(),
        ));
        assert!(has_stdio_upstreams(&cfg));

        let mut cfg = Config::default();
        cfg.servers.push(ServerConfig::new(
            "http".to_string(),
            "http://localhost:8080/mcp".parse().unwrap(),
        ));
        assert!(!has_stdio_upstreams(&cfg));
    }
}
pub(crate) use dev::DevCmd;
pub(crate) use init::InitCmd;
pub(crate) use list::ListCmd;
pub(crate) use remove::RemoveCmd;
pub(crate) use start::StartCmd;
