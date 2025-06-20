use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;
use url::Url;

fn disable_by_default() -> bool {
    false
}

fn random_by_default() -> u64 {
    rand::random()
}

/// Configuration for plugins, e.g. the simulator, error model, and runtime.
#[derive(Deserialize, Debug)]
pub struct PluginConfig {
    pub name: String,
    #[serde(default = "random_by_default")]
    pub seed: u64,
    pub file: PathBuf,
    pub args: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct EventHookConfig {
    #[serde(default = "disable_by_default")]
    pub provide_instruction_log: bool,
    #[serde(default = "disable_by_default")]
    pub provide_metrics: bool,
}

#[derive(Deserialize, Debug)]
pub struct ShotConfig {
    pub count: u64,
    pub offset: u64,
    pub increment: u64,
}

#[derive(Deserialize, Debug)]
pub struct Configuration {
    pub n_qubits: u64,
    pub output_stream: String,
    pub artifact_dir: PathBuf,
    pub simulator: PluginConfig,
    pub error_model: PluginConfig,
    pub runtime: PluginConfig,
    pub event_hooks: EventHookConfig,
    pub shots: ShotConfig,
}

impl Configuration {
    /// Get writer based on output arg (can be a file or a tcp address
    /// at the moment)
    pub fn get_output_writer(&self) -> Box<dyn std::io::Write> {
        match self.output_stream.as_str() {
            "stdout" => Box::new(std::io::stdout()),
            "stderr" => Box::new(std::io::stderr()),
            uri => {
                let url = Url::parse(uri).unwrap();
                match url.scheme() {
                    "file" => {
                        let path = url.path();
                        let file = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(path)
                            .unwrap();
                        Box::new(file)
                    }
                    "tcp" => {
                        let host = url.host_str().unwrap();
                        let port = url.port().unwrap();
                        let mut stream = std::net::TcpStream::connect((host, port)).unwrap();
                        // Communicate the shot configuration to the client
                        stream
                            .write_all(self.shots.offset.to_le_bytes().as_slice())
                            .unwrap();
                        stream
                            .write_all(self.shots.increment.to_le_bytes().as_slice())
                            .unwrap();
                        stream
                            .write_all(self.shots.count.to_le_bytes().as_slice())
                            .unwrap();
                        Box::new(stream)
                    }
                    _ => panic!("Unsupported output scheme: {}", url.scheme()),
                }
            }
        }
    }
}
