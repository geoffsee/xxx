use rcgen::generate_simple_self_signed;
use serde::Serialize;

pub fn make_cert() -> (String, String) {
    let subject_alt_names = vec!["localhost".to_string()];
    let cert_key_pair = generate_simple_self_signed(subject_alt_names)
        .expect("failed to generate self signed");

    let cert_pem = cert_key_pair.cert.pem();
    let key_pem = cert_key_pair.signing_key.serialize_pem();
    (cert_pem, key_pem)
}