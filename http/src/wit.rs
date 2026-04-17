wit_bindgen::generate!({
    world: "momento:http/imports",
    path: [ "wit" ],
    with: {
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
});
