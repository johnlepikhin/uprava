use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Page,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentRepresentation {
    Storage,
    Wiki,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentBodyStorage {
    pub value: String,
    pub representation: ContentRepresentation,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentBody {
    pub storage: ContentBodyStorage,
}

#[derive(Deserialize, Debug)]
#[serde(bound = "for<'de2> DATA: Deserialize<'de2>")]
pub struct PagedResult<DATA> {
    pub results: Vec<DATA>,
    pub start: usize,
    pub limit: usize,
    pub size: usize,
}
