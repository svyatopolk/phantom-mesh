use serde::{Serialize, Deserialize};
use serde_big_array::BigArray;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhantomFrame {
    pub stream_id: u32,
    pub noise_payload: Vec<u8>,
    #[serde(with = "BigArray")]
    pub signature: [u8; 64], // Ed25519 signature
}

impl PhantomFrame {
    pub fn new(stream_id: u32, payload: Vec<u8>, signature: [u8; 64]) -> Self {
        Self {
            stream_id,
            noise_payload: payload,
            signature,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Manual serialization or bincode/serde_json
        // Using serde_json for simplicity/debug now, switch to bincode for size later if needed.
        // User spec mentioned Protobuf. We can use manual bytes for "Structure".
        // Let's use simple manual packing for efficiency.
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.extend_from_slice(&(self.noise_payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.noise_payload);
        buf.extend_from_slice(&self.signature);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 4 + 4 + 64 { return None; }
        let mut offset = 0;
        
        let stream_id = u32::from_be_bytes(data[offset..offset+4].try_into().ok()?);
        offset += 4;
        
        let len = u32::from_be_bytes(data[offset..offset+4].try_into().ok()?) as usize;
        offset += 4;
        
        if data.len() < offset + len + 64 { return None; }
        
        let noise_payload = data[offset..offset+len].to_vec();
        offset += len;
        
        let signature = data[offset..offset+64].try_into().ok()?;
        
        Some(Self {
            stream_id,
            noise_payload,
            signature,
        })
    }
}
