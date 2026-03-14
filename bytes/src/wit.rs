wit_bindgen::generate!({
    world: "momento:bytes/imports",
    path: [ concat!(env!("OUT_DIR"), "/wit/host") ],
});
