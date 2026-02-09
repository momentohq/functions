mod function_web;
mod into_web_response;
mod response;
mod web_environment;
/// Internal module for WIT bindings.
#[doc(hidden)]
pub mod wit;

pub use function_web::invoke_template;
pub use into_web_response::IntoWebResponse;
pub use response::WebError;
pub use response::WebResponse;
pub use response::WebResult;
pub use web_environment::WebEnvironment;
