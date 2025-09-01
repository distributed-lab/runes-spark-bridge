use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[schema(example = json!({ }))]
pub struct Empty {}
