//! PostgreSQL認証用の小さな暗号部品(標準ライブラリのみ): MD5・SHA-256・HMAC・PBKDF2・SCRAM-SHA-256クライアント。
//! 試験はRFCの既知ベクトルで確認する。性能は問わない(接続時に1回だけ使う)。
use crate::DbError;

pub fn md5_hex(data: &[u8]) -> String {
    let s: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f2 = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f2.rotate_left(s[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|x| x.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7,
        0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
        0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
        0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut k = if key.len() > 64 { sha256(key).to_vec() } else { key.to_vec() };
    k.resize(64, 0);
    let mut inner: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(msg);
    let ih = sha256(&inner);
    let mut outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&ih);
    sha256(&outer)
}

pub fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut s = salt.to_vec();
    s.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha256(password, &s);
    let mut t = u;
    for _ in 1..iterations {
        u = hmac_sha256(password, &u);
        for i in 0..32 {
            t[i] ^= u[i];
        }
    }
    t
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub fn b64(data: &[u8]) -> String {
    let mut o = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        o.push(B64[(n >> 18) as usize & 63] as char);
        o.push(B64[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    o
}
pub fn unb64(s: &str) -> Result<Vec<u8>, DbError> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for ch in s.bytes().filter(|&c| c != b'=') {
        let v = B64.iter().position(|&x| x == ch).ok_or_else(|| DbError("base64が不正".into()))? as u32;
        buf = buf << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// SCRAM-SHA-256のクライアント(RFC 5802/7677。チャネルバインディング無し=`n,,`)。
pub struct Client {
    password: String,
    nonce: String,
    first_bare: String,
    server_sig: Option<[u8; 32]>,
}

fn random_nonce() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut raw = Vec::new();
    for i in 0..3u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        h.write_u64(i);
        raw.extend_from_slice(&h.finish().to_be_bytes());
    }
    b64(&raw)
}

impl Client {
    pub fn new(password: &str) -> Self {
        Self::with_nonce(password, &random_nonce())
    }
    pub fn with_nonce(password: &str, nonce: &str) -> Self {
        Client { password: password.into(), nonce: nonce.into(), first_bare: format!("n=,r={nonce}"), server_sig: None }
    }
    pub fn first_message(&self) -> String {
        format!("n,,{}", self.first_bare)
    }
    pub fn final_message(&mut self, server_first: &str) -> Result<String, DbError> {
        let get = |k: &str| server_first.split(',').find_map(|p| p.strip_prefix(k)).map(str::to_string);
        let r = get("r=").ok_or_else(|| DbError("サーバーの応答にr=がありません".into()))?;
        let s = get("s=").ok_or_else(|| DbError("サーバーの応答にs=がありません".into()))?;
        let i: u32 = get("i=").and_then(|x| x.parse().ok()).ok_or_else(|| DbError("サーバーの応答にi=がありません".into()))?;
        if !r.starts_with(&self.nonce) {
            return Err(DbError("サーバーのnonceが一致しません".into()));
        }
        if !(1..=10_000_000).contains(&i) {
            return Err(DbError("反復回数が範囲外です".into()));
        }
        let salted = pbkdf2_sha256(self.password.as_bytes(), &unb64(&s)?, i);
        let client_key = hmac_sha256(&salted, b"Client Key");
        let stored = sha256(&client_key);
        let without_proof = format!("c=biws,r={r}");
        let auth = format!("{},{server_first},{without_proof}", self.first_bare);
        let sig = hmac_sha256(&stored, auth.as_bytes());
        let proof: Vec<u8> = client_key.iter().zip(sig.iter()).map(|(a, b)| a ^ b).collect();
        let server_key = hmac_sha256(&salted, b"Server Key");
        self.server_sig = Some(hmac_sha256(&server_key, auth.as_bytes()));
        Ok(format!("{without_proof},p={}", b64(&proof)))
    }
    pub fn verify(&self, server_final: &str) -> Result<(), DbError> {
        let v = server_final.strip_prefix("v=").ok_or_else(|| DbError("サーバーの署名がありません".into()))?;
        let want = self.server_sig.ok_or_else(|| DbError("SCRAMの順序が不正".into()))?;
        if unb64(v)? == want { Ok(()) } else { Err(DbError("サーバーの署名が一致しません(接続先が不正かもしれません)".into())) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
    #[test]
    fn md5_known_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }
    #[test]
    fn sha256_and_hmac_known_vectors() {
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(hex(&hmac_sha256(b"key", b"The quick brown fox jumps over the lazy dog")), "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8");
    }
    #[test]
    fn pbkdf2_known_vector() {
        // RFC 7914 §11: PBKDF2-HMAC-SHA-256("passwd", "salt", 1)
        assert_eq!(hex(&pbkdf2_sha256(b"passwd", b"salt", 1)), "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc");
    }
    #[test]
    fn scram_rfc7677_example() {
        // RFC 7677 §3 の例(ユーザー user、パスワード pencil)
        let mut c = Client::with_nonce("pencil", "rOprNGfwEbeRWgbNEkqO");
        assert_eq!(c.first_message(), "n,,n=,r=rOprNGfwEbeRWgbNEkqO");
        c.first_bare = "n=user,r=rOprNGfwEbeRWgbNEkqO".into(); // RFC例のユーザー名に合わせる
        let sf = "r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096";
        let fin = c.final_message(sf).unwrap();
        assert_eq!(fin, "c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ=");
        c.verify("v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=").unwrap();
        assert!(c.verify("v=AAAA").is_err());
    }
    #[test]
    fn b64_round_trip() {
        for d in [&b""[..], b"a", b"ab", b"abc", b"abcd"] {
            assert_eq!(unb64(&b64(d)).unwrap(), d);
        }
        assert_eq!(b64(b"Man"), "TWFu");
    }
}
