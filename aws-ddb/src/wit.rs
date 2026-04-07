wit_bindgen::generate!({
    world: "momento:aws-ddb/imports",
    path: [ "wit" ],
    with: {
        "momento:aws-auth/aws-auth@1.0.0": momento_functions_aws_auth::wit::momento::aws_auth::aws_auth,
    },
});
