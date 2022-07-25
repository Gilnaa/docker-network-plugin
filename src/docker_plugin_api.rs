//! Structs used to deserialize libnetwork APIs.
//! - https://github.com/moby/libnetwork/blob/master/docs/remote.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct IpamData {
    pub address_space: String,
    pub pool: String,
    pub gateway: String,
    pub aux_addresses: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct InterfaceName {
    /// The name of the interface we create (on the root netns?)
    pub src_name: String,
    /// Prefix for the name of the interface inside the container.
    /// If we specify "eth", libnetwork will rename it to "eth<IDX>" (eth0, eth1, etc.)
    pub dst_prefix: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct StaticRoute {
    pub destination: String,
    /// Docs say:
    ///  > Routes are either given a RouteType of 0 and a value for NextHop;
    ///  > or, a RouteType of 1 and no value for NextHop, meaning a connected route.
    /// But who cares?
    pub route_type: i32,
    pub next_hop: Option<String>,
}

/**************************************************
 *                Top-Level Messages
 **************************************************/

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct HandshakeResponse {
    pub implements: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Capabilities {
    pub scope: String,
    pub connectivity_scope: String,
}

/// Struct definition taken from
///  https://pkg.go.dev/github.com/docker/go-plugins-helpers/network#AllocateNetworkRequest
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AllocateNetworkRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "IPv4Data")]
    pub ipv4_data: Option<Vec<IpamData>>,
    #[serde(rename = "IPv6Data")]
    pub ipv6_data: Option<Vec<IpamData>>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Struct definition taken from
///  https://pkg.go.dev/github.com/docker/go-plugins-helpers/network#AllocateNetworkResponse
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AllocateNetworkResponse {
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FreeNetworkRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CreateNetworkRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "IPv4Data")]
    pub ipv4_data: Option<Vec<IpamData>>,
    #[serde(rename = "IPv6Data")]
    pub ipv6_data: Option<Vec<IpamData>>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteNetworkRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct EndpointInterfaceInfo {
    pub address: String,
    #[serde(rename = "AddressIPv6")]
    pub address_ipv6: String,
    pub mac_address: String,
}

/// A request that is sent along with /NetworkDriver.CreateEndpoint
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CreateEndpointRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,

    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,

    pub options: Option<HashMap<String, serde_json::Value>>,

    pub interface: Option<EndpointInterfaceInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CreateEndpointResponse {
    pub interface: Option<EndpointInterfaceInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct EndpointOperInfoRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,

    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct EndpointOperInfoResponse {
    pub value: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DeleteEndpointRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct JoinRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
    pub sandbox_key: String,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct JoinResponse {
    pub interface_name: InterfaceName,

    pub gateway: Option<String>,

    #[serde(rename = "GatewayIPv6")]
    pub gateway_ipv6: Option<String>,

    pub sandbox_key: String,

    pub options: Option<HashMap<String, serde_json::Value>>,

    pub static_routes: Vec<StaticRoute>,

    pub disable_gateway_service: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct LeaveRequest {
    #[serde(rename = "NetworkID")]
    pub network_id: String,

    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
}
