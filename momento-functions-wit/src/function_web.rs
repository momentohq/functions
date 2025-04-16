wit_bindgen::generate!({
    world: "momento:functions/web-function",
    path: ["../wit/host/", "../wit/"],
    generate_all,
    default_bindings_module: "momento_functions_wit::function_web",
    export_macro_name: "export_web_function",
    pub_export_macro: true,
});
