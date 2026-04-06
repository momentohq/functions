wit_bindgen::generate!({
    world: "momento:cache-scalar/imports",
    path: [ concat!(env!("OUT_DIR"), "/wit/host") ],
    with: {
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
});
