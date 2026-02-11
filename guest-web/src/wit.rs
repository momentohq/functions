wit_bindgen::generate!({
    world: "momento:web-function/web-function",
    path: ["wit/host/", "wit/guest/"],
    with: {
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
    default_bindings_module: "momento_functions_guest_web::wit",
    export_macro_name: "export_web_function",
    pub_export_macro: true,
});
