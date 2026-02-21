use anyhow::{Result, Context, anyhow};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::core::Interface;

pub struct DesktopCapture {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    staging_texture: ID3D11Texture2D,
    desc: DXGI_OUTDUPL_DESC,
}

impl DesktopCapture {
    pub fn new() -> Result<Self> {
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().context("Failed to create DXGI factory")?;
            let adapter = factory.EnumAdapters1(0).context("Failed to find DXGI adapter")?;
            
            let mut device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;
            
            D3D11CreateDevice(
                &adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            ).context("Failed to create D3D11 device")?;
            
            let device = device.unwrap();
            let context = context.unwrap();
            
            let output = adapter.EnumOutputs(0).context("Failed to find DXGI output")?;
            let output1: IDXGIOutput1 = output.cast().context("Failed to cast to IDXGIOutput1")?;
            
            let duplication = output1.DuplicateOutput(&device).context("Failed to duplicate output")?;
            let desc = duplication.GetDesc();
            
            let texture_desc = D3D11_TEXTURE2D_DESC {
                Width: desc.ModeDesc.Width,
                Height: desc.ModeDesc.Height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            
            let mut staging_texture = None;
            device.CreateTexture2D(&texture_desc, None, Some(&mut staging_texture))
                .context("Failed to create staging texture")?;
            let staging_texture = staging_texture.unwrap();

            Ok(Self {
                device,
                context,
                duplication,
                staging_texture,
                desc,
            })
        }
    }

    pub fn capture_frame(&mut self) -> Result<Vec<u8>> {
        unsafe {
            let mut frame_resource: Option<IDXGIResource> = None;
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            
            // Acquire next frame (timeout 100ms)
            match self.duplication.AcquireNextFrame(100, &mut frame_info, &mut frame_resource) {
                Ok(_) => (),
                Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {
                    return Err(anyhow!("Capture timeout (no change)"));
                }
                Err(e) => return Err(e.into()),
            }

            // Optimization: Only process if LastPresentTime is non-zero
            if frame_info.LastPresentTime == 0 {
                self.duplication.ReleaseFrame().ok();
                return Err(anyhow!("Frame hasn't changed"));
            }
            
            let frame_resource = frame_resource.unwrap();
            let texture: ID3D11Texture2D = frame_resource.cast().context("Failed to cast resource to texture")?;
            
            // Copy to staging
            self.context.CopyResource(&self.staging_texture, &texture);
            
            self.duplication.ReleaseFrame().context("Failed to release frame")?;
            
            // Map and read
            let mut mapped_resource = D3D11_MAPPED_SUBRESOURCE::default();
            self.context.Map(&self.staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped_resource))
                .context("Failed to map staging texture")?;
            
            let width = self.desc.ModeDesc.Width as usize;
            let height = self.desc.ModeDesc.Height as usize;
            let row_pitch = mapped_resource.RowPitch as usize;
            
            let mut buffer = Vec::with_capacity(width * height * 4);
            let ptr = mapped_resource.pData as *const u8;
            
            for y in 0..height {
                let row_start = ptr.add(y * row_pitch);
                let row_slice = std::slice::from_raw_parts(row_start, width * 4);
                buffer.extend_from_slice(row_slice);
            }
            
            self.context.Unmap(&self.staging_texture, 0);
            
            Ok(buffer)
        }
    }

    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.desc.ModeDesc.Width, self.desc.ModeDesc.Height)
    }
}
