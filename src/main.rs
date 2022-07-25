use std::{collections::HashMap, path::Path, sync::Arc};

use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use docker_plugin_api::{
    AllocateNetworkRequest, AllocateNetworkResponse, Capabilities, CreateEndpointRequest,
    CreateNetworkRequest, DeleteEndpointRequest, DeleteNetworkRequest, EndpointOperInfoRequest,
    EndpointOperInfoResponse, FreeNetworkRequest, JoinRequest, JoinResponse, LeaveRequest,
};
use futures::stream::TryStreamExt;
use permissive_json::PermissiveJson;
use tokio::sync::Mutex;

use axum::{
    extract::Extension,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use hyper::StatusCode;
use hyperlocal::UnixServerExt;
mod docker_plugin_api;
mod permissive_json;

struct PluginError {
    error_code: StatusCode,
    error_string: String,
}

impl PluginError {
    fn new(error_code: StatusCode, error_string: impl ToString) -> Self {
        Self {
            error_code,
            error_string: error_string.to_string(),
        }
    }
}

impl<S: ToString> From<S> for PluginError {
    fn from(error_string: S) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error_string.to_string())
    }
}

impl IntoResponse for PluginError {
    fn into_response(self) -> Response {
        (self.error_code, self.error_string).into_response()
    }
}

struct Network {
    bridge_name: String,
    bridge_index: u32,
    endpoints: Vec<String>,
}

impl Network {
    fn new(bridge_name: String, bridge_index: u32) -> Self {
        Self {
            bridge_name,
            bridge_index,
            endpoints: Vec::new(),
        }
    }
}

struct AppState {
    /// Created networks, keyed by NetworkID
    networks: HashMap<String, Network>,
}

type AppStateHandle = Arc<Mutex<AppState>>;

impl AppState {
    fn new_handle() -> AppStateHandle {
        Arc::new(Mutex::new(AppState {
            networks: HashMap::new(),
        }))
    }
}

fn get_endpoint_veth_pair_names(endpoint_id: &str) -> (String, String) {
    (
        format!("v{}", &endpoint_id[..13]),
        format!("V{}", &endpoint_id[..13]),
    )
}

async fn plugin_activate() {}

async fn handle_get_capabilities() -> impl IntoResponse {
    let caps = Capabilities {
        scope: "global".into(),
        connectivity_scope: "global".into(),
    };

    (StatusCode::OK, Json(caps))
}

async fn handle_allocate_network(
    PermissiveJson(request): PermissiveJson<AllocateNetworkRequest>,
) -> impl IntoResponse {
    let response = AllocateNetworkResponse {
        options: request.options,
    };

    (StatusCode::OK, Json(response))
}

async fn handle_free_network(
    PermissiveJson(request): PermissiveJson<FreeNetworkRequest>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(()))
}

async fn handle_create_network(
    PermissiveJson(request): PermissiveJson<CreateNetworkRequest>,
    Extension(app_state): Extension<AppStateHandle>,
    Extension(rtnl_handle): Extension<rtnetlink::Handle>,
) -> Result<Json<()>, PluginError> {
    let mut app_state = app_state.lock().await;
    if app_state.networks.contains_key(&request.network_id) {
        Err(format!(
            "Plugin already contains a network for {}",
            request.network_id
        ))?;
    }

    let bridge_name = request
        .options
        .as_ref()
        .and_then(|opts| opts.get("bridge_name"));
    let bridge_name = match bridge_name {
        None => Ok("florp"),
        Some(serde_json::Value::String(name)) => Ok(name.as_str()),
        Some(invalid) => Err(format!("Invalid variant for bridge: {invalid}")),
    }?;

    let message = rtnl_handle.link().add().bridge(bridge_name.into());
    message.execute().await.map_err(|err| {
        let err_msg = format!("Failed creating bridge: {err}");
        eprintln!("{}", err_msg);
        err_msg
    })?;
    let mut links = rtnl_handle
        .link()
        .get()
        .match_name(bridge_name.into())
        .execute();
    let link = links.try_next().await?.ok_or_else(|| {
        format!(
            "Could linux interface index for bridge {} of network {}",
            bridge_name, request.network_id
        )
    })?;
    let idx = link.header.index;

    app_state
        .networks
        .insert(request.network_id, Network::new(bridge_name.into(), idx));
    rtnl_handle
        .link()
        .set(idx)
        .up()
        .execute()
        .await
        .map_err(|err| format!("Failed setting bridge up {err}"))?;

    Ok(Json(()))
}

async fn handle_delete_network(
    PermissiveJson(request): PermissiveJson<DeleteNetworkRequest>,
    Extension(app_state): Extension<AppStateHandle>,
    Extension(rtnl_handle): Extension<rtnetlink::Handle>,
) -> Result<Json<()>, PluginError> {
    let mut app_state = app_state.lock().await;
    let network = app_state
        .networks
        .get(&request.network_id)
        .ok_or_else(|| format!("Could not find network {}", &request.network_id))?;

    rtnl_handle
        .link()
        .del(network.bridge_index)
        .execute()
        .await
        .map_err(|err| {
            format!(
                "Failed deleting link {}:{}: {err}",
                network.bridge_name, network.bridge_index
            )
        })?;

    app_state.networks.remove(&request.network_id);

    Ok(Json(()))
}

async fn handle_create_endpoint(
    PermissiveJson(request): PermissiveJson<CreateEndpointRequest>,
    Extension(app_state): Extension<AppStateHandle>,
    Extension(rtnl_handle): Extension<rtnetlink::Handle>,
) -> Result<Json<()>, PluginError> {
    let mut app_state = app_state.lock().await;
    let network = app_state
        .networks
        .get_mut(&request.network_id)
        .ok_or_else(|| format!("Could not find network {}", &request.network_id))?;
    let (veth_name, peer_name) = get_endpoint_veth_pair_names(&request.endpoint_id);

    rtnl_handle
        .link()
        .add()
        .veth(veth_name.clone(), peer_name.clone())
        .execute()
        .await
        .map_err(|err| {
            format!(
                "Failed creating veths for network/endpoint {}/{}: {err}",
                request.network_id, request.endpoint_id
            )
        })?;
    let mut links = rtnl_handle
        .link()
        .get()
        .match_name(veth_name.clone())
        .execute();
    let link = links
        .try_next()
        .await?
        .ok_or_else(|| format!("Could linux interface index for veth {veth_name}"))?;
    let veth_index = link.header.index;

    rtnl_handle
        .link()
        .set(veth_index)
        .master(network.bridge_index)
        .up()
        .execute()
        .await
        .map_err(|err| {
            format!(
                "Failed deleting link {}:{}: {err}",
                network.bridge_name, network.bridge_index
            )
        })?;

    Ok(Json(()))
}

async fn handle_endpoint_oper_info(
    PermissiveJson(request): PermissiveJson<EndpointOperInfoRequest>,
) -> Result<Json<EndpointOperInfoResponse>, PluginError> {
    Ok(Json(EndpointOperInfoResponse {
        value: HashMap::new(),
    }))
}

async fn handle_delete_endpoint(
    PermissiveJson(request): PermissiveJson<DeleteEndpointRequest>,
) -> Result<Json<()>, PluginError> {
    Ok(Json(()))
}

async fn handle_join(PermissiveJson(request): PermissiveJson<JoinRequest>) -> Json<JoinResponse> {
    let (_veth_name, peer_name) = get_endpoint_veth_pair_names(&request.endpoint_id);
    let join_response = JoinResponse {
        interface_name: docker_plugin_api::InterfaceName {
            src_name: peer_name,
            dst_prefix: "eth".into(),
        },
        gateway: None,
        gateway_ipv6: None,
        sandbox_key: request.sandbox_key,
        options: request.options,
        disable_gateway_service: false,
        static_routes: vec![],
    };
    Json(join_response)
}

async fn handle_leave(PermissiveJson(request): PermissiveJson<LeaveRequest>) -> Json<()> {
    Json(())
}

#[tokio::main]
async fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "example_static_file_server=debug,tower_http=debug".into()),
        ))
        .with(fmt_layer)
        .init();

    eprintln!("Starting network plugin");

    let socket_path = Path::new("/run/docker/plugins/network.sock");
    if socket_path.exists() {
        std::fs::remove_file(socket_path).unwrap();
    }

    // Start the netlink task in the background.
    let (connection, handle, _message_queue) = rtnetlink::new_connection().unwrap();
    tokio::spawn(connection);
    let app_state = AppState::new_handle();

    let app = Router::new()
        .route("/Plugin.Activate", post(plugin_activate))
        .route(
            "/NetworkDriver.GetCapabilities",
            post(handle_get_capabilities),
        )
        .route(
            "/NetworkDriver.AllocateNetwork",
            post(handle_allocate_network),
        )
        .route("/NetworkDriver.FreeNetwork", post(handle_free_network))
        .route("/NetworkDriver.CreateNetwork", post(handle_create_network))
        .route("/NetworkDriver.DeleteNetwork", post(handle_delete_network))
        .route(
            "/NetworkDriver.CreateEndpoint",
            post(handle_create_endpoint),
        )
        .route(
            "/NetworkDriver.EndpointOperInfo",
            post(handle_endpoint_oper_info),
        )
        .route(
            "/NetworkDriver.DeleteEndpoint",
            post(handle_delete_endpoint),
        )
        .route("/NetworkDriver.Join", post(handle_join))
        .route("/NetworkDriver.Leave", post(handle_leave))
        .layer(Extension(app_state))
        .layer(Extension(handle))
        .layer(TraceLayer::new_for_http());
    axum::Server::bind_unix(socket_path)
        .unwrap()
        .serve(app.into_make_service())
        .await
        .unwrap();
}
