wit_bindgen::generate!({
    world: "momento:functions/web-function",
    path: ["wit/host/", "wit/"],
    generate_all,
    default_bindings_module: "momento_functions_wit::function_web",
    export_macro_name: "export_web_function",
    pub_export_macro: true,
});

impl From<(String, String)>
    for crate::function_web::momento::functions::web_function_support::Header
{
    fn from((name, value): (String, String)) -> Self {
        Self { name, value }
    }
}
