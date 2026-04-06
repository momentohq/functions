wit_bindgen::generate!({
    world: "momento:log/imports",
    path: [concat!(env!("OUT_DIR"), "/wit/host")],
});
