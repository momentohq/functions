wit_bindgen::generate!({
    world: "momento:aws-auth/imports",
    path: [ concat!(env!("OUT_DIR"), "/wit/host") ],
});
