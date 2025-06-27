use anyhow::Result;
use openssl::symm::{Cipher, Crypter, Mode};
use rand::RngCore;
use std::{
    io::{Read, Write},
    path::Path,
};

const TTY_TAG: u32 = 0x5954544D;
const G_IV: &[u8] = &[
    0x27, 0xD5, 0x15, 0xF7, 0x34, 0xD4, 0x57, 0xBD, 0x9C, 0xB3, 0xA3, 0xDF, 0xC1, 0xA5, 0x47, 0x57,
];

pub fn pass_to_key(user_name: &str, pass: &str) -> Vec<u8> {
    let lw = user_name.to_lowercase();

    let mut uname: Vec<u8> = Vec::with_capacity(lw.len() * 2);
    let mut upass: Vec<u8> = Vec::with_capacity(pass.len() * 2);

    for v in lw.as_bytes() {
        uname.write_all(&(*v as u16).to_le_bytes()).ok();
    }

    for v in pass.as_bytes() {
        upass.write_all(&(*v as u16).to_le_bytes()).ok();
    }

    let mut ctx = md5::Context::new();
    ctx.consume(&uname);
    ctx.consume(G_IV);
    ctx.consume(&upass);
    let d = ctx.finalize();

    d.to_vec()
}

fn decode(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != G_IV.len() {
        anyhow::bail!("invalid key length")
    }

    let mut iv = Vec::with_capacity(G_IV.len());
    for (i, k) in G_IV.iter().zip(key.iter()) {
        iv.push(*k ^ *i);
    }

    let mut crypter = Crypter::new(Cipher::sm4_cfb128(), Mode::Decrypt, key, Some(&iv))?;

    let mut output = vec![0_u8; data.len() + 16];
    let buf = output.as_mut_slice();

    let mut idx = crypter.update(data, buf)?;

    idx += crypter.finalize(&mut buf[idx..])?;

    output.truncate(idx);

    Ok(output)
}

fn encode(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != G_IV.len() {
        anyhow::bail!("invalid key length")
    }

    let mut iv = Vec::with_capacity(G_IV.len());
    for (i, k) in G_IV.iter().zip(key.iter()) {
        iv.push(*k ^ *i);
    }

    let mut crypter = Crypter::new(Cipher::sm4_cfb128(), Mode::Encrypt, key, Some(&iv))?;

    let mut output = vec![0_u8; data.len() + 16];
    let buf = output.as_mut_slice();

    let mut idx = crypter.update(data, buf)?;

    idx += crypter.finalize(&mut buf[idx..])?;

    output.truncate(idx);
    Ok(output)
}

pub fn verify_password<P: AsRef<Path>>(
    file_name: P,
    name: &str,
    pass: &str,
) -> Result<(Vec<u8>, Vec<u8>)> {
    if !file_name.as_ref().exists() {
        let mut data_key = vec![0_u8; 16];
        let buf = data_key.as_mut_slice();
        let mut rng = rand::rng();
        rng.fill_bytes(buf);
        return Ok((pass_to_key(name, pass), data_key));
    }

    let mut file = std::fs::File::open(&file_name)?;
    let mut data = vec![0_u8; 24];
    let buf = data.as_mut_slice();

    let n = file.read(buf)?;
    if n != buf.len() {
        anyhow::bail!("invalid file {}", file_name.as_ref().display());
    }

    let key = pass_to_key(name, pass);

    let d = decode(buf, &key)?;

    let tag = u32::from_le_bytes(d[0..4].try_into().unwrap_or_default());
    if tag != TTY_TAG {
        anyhow::bail!("invalid username or password tag:{}", tag);
    }

    Ok((key, d[8..].to_vec()))
}

pub fn load_server<P: AsRef<Path>>(
    file_name: P,
    user_key: &[u8],
    data_key: &[u8],
) -> Result<String> {
    let data = std::fs::read(&file_name)?;

    if data.len() < 24 {
        anyhow::bail!("invalid file {}", file_name.as_ref().display());
    }

    let buf = data.as_slice();
    let head = &buf[..24];

    let hd = decode(head, user_key)?;
    let tag = u32::from_le_bytes(hd[0..4].try_into().unwrap_or_default());

    if tag != TTY_TAG {
        anyhow::bail!("invalid username or password tag:{}", tag);
    }

    if data_key != &hd[8..24] {
        anyhow::bail!("invalid data key");
    }

    let data = decode(&buf[24..], data_key)?;

    Ok(String::from_utf8(data)?)
}

pub fn save_server<P: AsRef<Path>>(
    file_name: P,
    user_key: &[u8],
    data_key: &[u8],
    data: &str,
) -> Result<()> {
    let mut file = std::fs::File::create(&file_name)?;
    let mut hd: Vec<u8> = Vec::with_capacity(24);
    let ed = encode(data.as_bytes(), data_key)?;

    // TAG
    hd.write_all(&TTY_TAG.to_le_bytes())?;

    // SIZE
    let d_size = ed.len() as u32;
    hd.write_all(&d_size.to_le_bytes())?;

    // data key
    hd.write_all(data_key)?;

    let ehd = encode(&hd, user_key)?;

    file.write_all(&ehd)?;
    file.write_all(&ed)?;

    Ok(())
}
