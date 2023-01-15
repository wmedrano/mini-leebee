use std::{collections::HashSet, sync::RwLock};

use audio_engine::commands::Command;
use mini_leebee_proto::{
    AddPluginToTrackRequest, AddPluginToTrackResponse, CreateTrackRequest, CreateTrackResponse,
    DeleteTracksRequest, DeleteTracksResponse, GetPluginsRequest, GetPluginsResponse,
    GetTracksRequest, GetTracksResponse, Plugin, Track, TrackPlugin,
};
use tonic::{Request, Response, Status};

/// Implements the MiniLeebee gRPC service.
#[derive(Debug)]
pub struct MiniLeebeeServer {
    jack_adapter: jack_adapter::JackAdapter,
    state: RwLock<State>,
}

#[derive(Debug)]
struct State {
    tracks: Vec<Track>,
    next_track_id: i32,
}

impl MiniLeebeeServer {
    /// Create a new server.
    pub fn new(jack_adapter: jack_adapter::JackAdapter) -> MiniLeebeeServer {
        MiniLeebeeServer {
            jack_adapter,
            state: RwLock::new(State {
                tracks: Vec::new(),
                next_track_id: 1,
            }),
        }
    }
}

#[tonic::async_trait]
impl mini_leebee_proto::mini_leebee_server::MiniLeebee for MiniLeebeeServer {
    /// Get the plugins.
    async fn get_plugins(
        &self,
        _: Request<GetPluginsRequest>,
    ) -> Result<Response<GetPluginsResponse>, Status> {
        let plugins = self
            .jack_adapter
            .audio_engine
            .livi
            .iter_plugins()
            .map(|p| Plugin {
                id: id_for_plugin(&p),
                name: p.name(),
            })
            .collect();
        Ok(Response::new(GetPluginsResponse { plugins }))
    }

    /// Add a plugin to a track.
    async fn add_plugin_to_track(
        &self,
        request: Request<AddPluginToTrackRequest>,
    ) -> Result<Response<AddPluginToTrackResponse>, Status> {
        let request = request.into_inner();
        let plugin = match self
            .jack_adapter
            .audio_engine
            .livi
            .iter_plugins()
            .find(|p| id_for_plugin(p) == request.plugin_id)
        {
            Some(p) => p,
            None => {
                return Err(Status::not_found(format!(
                    "plugin {} not found",
                    request.plugin_id
                )));
            }
        };
        let mut state = self.state.write().unwrap();
        let track = match state.tracks.iter_mut().find(|t| t.id == request.track_id) {
            Some(t) => t,
            None => {
                return Err(Status::not_found(format!(
                    "track {} not found",
                    request.track_id
                )));
            }
        };
        let instance_or_err = unsafe {
            plugin.instantiate(
                self.jack_adapter.audio_engine.lv2_features.clone(),
                self.jack_adapter.sample_rate(),
            )
        };
        let instance = match instance_or_err {
            Ok(i) => i,
            Err(err) => {
                return Err(Status::internal(format!(
                    "failed to instantiate plugin {}: {}",
                    plugin.name(),
                    err
                )))
            }
        };
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::AddPluginToTrack(track.id, instance))
            .unwrap();
        track.plugins.push(TrackPlugin {
            plugin_id: request.plugin_id,
        });
        Ok(Response::new(AddPluginToTrackResponse {}))
    }

    /// Get the tracks.
    async fn get_tracks(
        &self,
        _: Request<GetTracksRequest>,
    ) -> Result<Response<GetTracksResponse>, Status> {
        let tracks = self.state.read().unwrap().tracks.clone();
        Ok(Response::new(GetTracksResponse { tracks }))
    }

    /// Create a track.
    async fn create_track(
        &self,
        request: Request<CreateTrackRequest>,
    ) -> Result<Response<CreateTrackResponse>, Status> {
        let mut state = self.state.write().unwrap();
        let track_id = state.next_track_id;
        let track = Track {
            name: replace_if_default(request.into_inner().name, || format!("Track {}", track_id)),
            id: track_id,
            plugins: Vec::new(),
        };
        let audio_engine_track =
            audio_engine::track::Track::new(track_id, self.jack_adapter.buffer_size());
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::AddTrack(audio_engine_track))
            .unwrap();
        state.tracks.push(track);
        state.next_track_id += 1;
        Ok(Response::new(CreateTrackResponse { track_id }))
    }

    /// Delete tracks.
    async fn delete_tracks(
        &self,
        request: Request<DeleteTracksRequest>,
    ) -> Result<Response<DeleteTracksResponse>, Status> {
        let ids_requested_for_deletion: HashSet<i32> =
            request.into_inner().track_ids.drain(..).collect();
        let deleted_track_ids = {
            let mut state = self.state.write().unwrap();
            let existing_ids = state.tracks.iter().map(|t| t.id);
            let delete_targets: HashSet<i32> = existing_ids
                .filter(|id| ids_requested_for_deletion.contains(id))
                .collect();
            for t in delete_targets.iter() {
                self.jack_adapter
                    .audio_engine
                    .commands
                    .send(Command::DeleteTrack(*t))
                    .unwrap();
            }
            state.tracks.retain(|t| !delete_targets.contains(&t.id));
            delete_targets
        };
        Ok(Response::new(DeleteTracksResponse {
            deleted_track_ids: deleted_track_ids.into_iter().collect(),
        }))
    }
}

/// Get the result of `f` if `t` is equal to its default value. If not, then `t`
/// is returned.
fn replace_if_default<T: Default + std::cmp::PartialEq, F: Fn() -> T>(t: T, f: F) -> T {
    if t == T::default() {
        f()
    } else {
        t
    }
}

/// Get the id for the plugin.
fn id_for_plugin(p: &livi::Plugin) -> String {
    format!("lv2:{}", p.uri())
}
