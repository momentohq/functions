wit_bindgen::generate!({
    world: "momento:cache-scalar/imports",
    path: [ "wit" ],
    with: {
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
});
