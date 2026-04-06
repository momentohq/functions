wit_bindgen::generate!({
    world: "momento:aws-s3/imports",
    path: [ concat!(env!("OUT_DIR"), "/wit/host") ],
    with: {
        "momento:aws-auth/aws-auth@1.0.0": momento_functions_aws_auth::wit::momento::aws_auth::aws_auth,
        "momento:bytes/bytes@1.0.0": momento_functions_bytes::wit::momento::bytes::bytes,
    },
});
