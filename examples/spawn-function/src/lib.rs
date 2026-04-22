use momento_functions_bytes::Data;
use momento_functions_guest_spawn::spawn;

spawn!(spawned);
fn spawned(_payload: Data) {}
