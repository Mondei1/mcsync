use serde::{Serialize, Deserialize};

// Pasted from client

#[derive(Serialize, Deserialize)]
pub struct CreateServer {
    server_name: String
}

#[derive(Serialize, Deserialize)]
pub struct CreateServerResponse {
    server_uuid: String
}

// End pasted from client

// #[post("/server", data = "<create_server>")]
// pub fn create_server(create_server: Json<CreateServer>) -> String {
//     create_server.server_name.clone()
// }
//
// #[post("/server/<id>/transfer", data = "<data>")]
// pub fn transfer(id: String, data: Data) -> Result<String, std::io::Error> {
//     Ok(String::from("progress"))
// }
