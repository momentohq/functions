wit_bindgen::generate!({
    world: "momento:spawn-function/spawn-function",
    path: [
        concat!(env!("OUT_DIR"), "/wit/host"),
        concat!(env!("OUT_DIR"), "/wit/guest"),
    ],
    with: {
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
    default_bindings_module: "momento_functions_guest_spawn::wit",
    export_macro_name: "export_spawn_function",
    pub_export_macro: true,
});
