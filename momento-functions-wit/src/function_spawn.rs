wit_bindgen::generate!({
    world: "momento:functions/spawn-function",
    path: ["../wit/host/", "../wit/"],
    generate_all,
    default_bindings_module: "momento_functions_wit::function_spawn",
    export_macro_name: "export_spawn_function",
    pub_export_macro: true,
});
