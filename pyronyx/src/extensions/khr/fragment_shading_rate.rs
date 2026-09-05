// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// Auto generated from pyronyx-gen — generated extensions
// Do not Edit! Execute `cargo run pyronyx-gen`
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

use crate::vk::*;
use core::ffi::CStr;
use core::mem::MaybeUninit;
use core::ptr;

/// Type: `Device`
pub const NAME: &CStr = c"VK_KHR_fragment_shading_rate";
pub const SPEC_VERSION: u32 = 2;

pub trait FragmentShadingRateCommandBuffer {
    fn set_fragment_shading_rate(
        &self,
        fragment_size: &Extent2D,
        combiner_ops: FragmentShadingRateCombinerOpKHR,
    );
}

impl FragmentShadingRateCommandBuffer for CommandBuffer {
    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdSetFragmentShadingRateKHR.html>
    ///
    /// Queues types: `Graphics`.
    /// Task: `Vulkan state access`.
    /// Use inside and outside `RenderPass`.
    /// Command buffer level: `primary`, `secondary`.
    #[inline]
    fn set_fragment_shading_rate(
        &self,
        fragment_size: &Extent2D,
        combiner_ops: FragmentShadingRateCombinerOpKHR,
    ) {
        let call = self
            .fns()
            .khr_fragment_shading_rate
            .as_ref()
            .expect(Self::EXT_LOAD_ERROR)
            .set_fragment_shading_rate_khr;

        unsafe { (call)(self.handle, fragment_size, combiner_ops) };
    }
}

pub trait FragmentShadingRatePhysicalDevice {
    fn get_fragment_shading_rates(
        &self,
        fragment_shading_rates: &mut [PhysicalDeviceFragmentShadingRateKHR],
    ) -> Result<()>;
    fn get_fragment_shading_rates_len(&self) -> Result<usize>;
}

impl FragmentShadingRatePhysicalDevice for PhysicalDevice {
    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkGetPhysicalDeviceFragmentShadingRatesKHR.html>
    ///
    /// Call [`get_fragment_shading_rates_len()`][`Self::get_fragment_shading_rates_len()`] to query the number of elements to pass to `out`.
    #[inline]
    fn get_fragment_shading_rates(
        &self,
        fragment_shading_rates: &mut [PhysicalDeviceFragmentShadingRateKHR],
    ) -> Result<()> {
        let mut fragment_shading_rate_count = fragment_shading_rates.len() as u32;
        let call = self
            .fns()
            .khr_fragment_shading_rate
            .as_ref()
            .expect(Self::EXT_LOAD_ERROR)
            .get_physical_device_fragment_shading_rates_khr;

        unsafe {
            (call)(
                self.handle,
                &mut fragment_shading_rate_count,
                fragment_shading_rates.as_mut_ptr(),
            )
        }
        .result()
    }

    /// Returns the required slice length for Call [`get_fragment_shading_rates`][`Self::get_fragment_shading_rates`].
    #[inline]
    fn get_fragment_shading_rates_len(&self) -> Result<usize> {
        let mut out: MaybeUninit<u32> = MaybeUninit::uninit();
        unsafe {
            (self
                .fns()
                .khr_fragment_shading_rate
                .as_ref()
                .expect(Self::EXT_LOAD_ERROR)
                .get_physical_device_fragment_shading_rates_khr)(
                self.handle,
                out.as_mut_ptr(),
                ptr::null_mut(),
            )
        }
        .init_on_success(out)
        .map(|v| v as usize)
    }
}
