use mini_leebee_proto::{GetPluginsRequest, GetPluginsResponse};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct MiniLeebeeServer {
    jack_adapter: jack_adapter::JackAdapter,
}

impl MiniLeebeeServer {
    pub fn new(jack_adapter: jack_adapter::JackAdapter) -> MiniLeebeeServer {
        MiniLeebeeServer { jack_adapter }
    }
}

#[tonic::async_trait]
impl mini_leebee_proto::mini_leebee_server::MiniLeebee for MiniLeebeeServer {
    async fn get_plugins(
        &self,
        _: Request<GetPluginsRequest>,
    ) -> Result<Response<GetPluginsResponse>, Status> {
        let plugins = self
            .jack_adapter
            .audio_engine
            .livi
            .iter_plugins()
            .map(|p| mini_leebee_proto::Plugin {
                id: format!("lv2:{}", p.uri()),
                name: p.name(),
            })
            .collect();
        Ok(Response::new(GetPluginsResponse { plugins }))
    }
}
