wit_bindgen::generate!({
    world: "momento:spawn-function/spawn-function",
    path: [
        "wit",
    ],
    with: {
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
    default_bindings_module: "momento_functions_guest_spawn::wit",
    export_macro_name: "export_spawn_function",
    pub_export_macro: true,
});
