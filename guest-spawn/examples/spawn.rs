use momento_functions_bytes::Data;
use momento_functions_guest_spawn::spawn;

spawn!(triggered);
fn triggered(_payload: Data) {}
