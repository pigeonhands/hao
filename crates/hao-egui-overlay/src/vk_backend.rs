//! Vulkan overlay: hooks swapchain creation and present to inject egui
//! rendering before the frame is shown.
//!
//! Hooks are installed by exporting `vkGetInstanceProcAddr`. The game loads
//! Vulkan through us, and we intercept key functions while forwarding the rest.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::sync::Mutex;

use ash::vk;

use crate::input;

static REAL_GET_INSTANCE_PROC: Mutex<Option<vk::PFN_vkGetInstanceProcAddr>> = Mutex::new(None);
static REAL_GET_DEVICE_PROC: Mutex<Option<vk::PFN_vkGetDeviceProcAddr>> = Mutex::new(None);
static STATE: Mutex<Option<VkOverlay>> = Mutex::new(None);
static DEVICE_MAP: Mutex<Option<DeviceState>> = Mutex::new(None);

struct DeviceState {
    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queue: vk::Queue,
    queue_family: u32,
    swapchain_fn: ash::khr::swapchain::Device,
    real_queue_present: vk::PFN_vkQueuePresentKHR,
    real_create_swapchain: vk::PFN_vkCreateSwapchainKHR,
    real_destroy_swapchain: vk::PFN_vkDestroySwapchainKHR,
}

unsafe impl Send for DeviceState {}

struct VkOverlay {
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    sampler: vk::Sampler,
    command_pool: vk::CommandPool,

    textures: HashMap<egui::TextureId, ManagedTexture>,

    framebuffers: Vec<vk::Framebuffer>,
    image_views: Vec<vk::ImageView>,
    command_buffers: Vec<vk::CommandBuffer>,
    fences: Vec<vk::Fence>,
    semaphores: Vec<vk::Semaphore>,

    extent: vk::Extent2D,
    #[allow(dead_code)]
    format: vk::Format,

    vertex_buffers: Vec<Option<VkBuffer>>,
    index_buffers: Vec<Option<VkBuffer>>,

    ctx: egui::Context,
    pointer_down: [bool; 2],
    keys_down: [bool; input::KEY_COUNT],
}

struct VkTexture {
    image: vk::Image,
    view: vk::ImageView,
    memory: vk::DeviceMemory,
}

struct ManagedTexture {
    vk_tex: VkTexture,
    descriptor_set: vk::DescriptorSet,
    atlas: Vec<u8>,
    size: [usize; 2],
}

struct VkBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
}

pub fn install() {
    tracing::info!("hao-egui-overlay: Vulkan hooks ready (via symbol export)");
}

/// Look up a Vulkan symbol, handling the RTLD_LOCAL case where libvulkan
/// was dlopen'd without RTLD_GLOBAL (Godot, etc.).
unsafe fn vk_dlsym(symbol: *const c_char) -> *mut c_void {
    let sym = libc::dlsym(libc::RTLD_NEXT, symbol);
    if !sym.is_null() {
        return sym;
    }
    let lib = libc::dlopen(b"libvulkan.so.1\0".as_ptr() as _, libc::RTLD_NOW | libc::RTLD_NOLOAD);
    if lib.is_null() {
        let lib = libc::dlopen(b"libvulkan.so.1\0".as_ptr() as _, libc::RTLD_NOW);
        if lib.is_null() {
            return std::ptr::null_mut();
        }
        #[cfg(feature = "hooking")]
        let sym = super::hook::real_dlsym(lib, symbol);
        #[cfg(not(feature = "hooking"))]
        let sym = libc::dlsym(lib, symbol);
        libc::dlclose(lib);
        return sym;
    }
    #[cfg(feature = "hooking")]
    let sym = super::hook::real_dlsym(lib, symbol);
    #[cfg(not(feature = "hooking"))]
    let sym = libc::dlsym(lib, symbol);
    libc::dlclose(lib);
    sym
}

fn get_real_get_instance_proc() -> Option<vk::PFN_vkGetInstanceProcAddr> {
    let mut guard = REAL_GET_INSTANCE_PROC.lock().unwrap();
    if let Some(f) = *guard {
        return Some(f);
    }

    let sym = unsafe {
        libc::dlsym(libc::RTLD_NEXT, b"vkGetInstanceProcAddr\0".as_ptr() as _)
    };
    if !sym.is_null() {
        let f = unsafe { std::mem::transmute(sym) };
        *guard = Some(f);
        return Some(f);
    }

    // Godot and other engines dlopen libvulkan with RTLD_LOCAL,
    // so RTLD_NEXT can't see it. Try NOLOAD first, then a real load.
    for &flags in &[libc::RTLD_NOW | libc::RTLD_NOLOAD, libc::RTLD_NOW] {
        let lib = unsafe {
            libc::dlopen(b"libvulkan.so.1\0".as_ptr() as _, flags)
        };
        if lib.is_null() {
            continue;
        }
        #[cfg(feature = "hooking")]
        let sym = unsafe {
            super::hook::real_dlsym(lib, b"vkGetInstanceProcAddr\0".as_ptr() as _)
        };
        #[cfg(not(feature = "hooking"))]
        let sym = unsafe { libc::dlsym(lib, b"vkGetInstanceProcAddr\0".as_ptr() as _) };
        unsafe { libc::dlclose(lib); }
        if !sym.is_null() {
            let f = unsafe { std::mem::transmute(sym) };
            *guard = Some(f);
            return Some(f);
        }
    }

    None
}

fn find_memory_type(
    props: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    flags: vk::MemoryPropertyFlags,
) -> u32 {
    for i in 0..props.memory_type_count {
        if type_bits & (1 << i) != 0
            && props.memory_types[i as usize]
                .property_flags
                .contains(flags)
        {
            return i;
        }
    }
    panic!("hao-egui-overlay: no suitable memory type");
}

impl VkOverlay {
    unsafe fn new(ds: &DeviceState, images: &[vk::Image], format: vk::Format, extent: vk::Extent2D) -> Self {
        let device = &ds.device;

        // Render pass
        let attachment = vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_ref));

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let rp_info = vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&attachment))
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dependency));
        let render_pass = device.create_render_pass(&rp_info, None).unwrap();

        // Descriptor set layout (one combined image sampler)
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let dsl_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(std::slice::from_ref(&binding));
        let descriptor_set_layout = device
            .create_descriptor_set_layout(&dsl_info, None)
            .unwrap();

        // Pipeline layout (push constant: vec2 screen_size)
        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(8);

        let pl_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(&descriptor_set_layout))
            .push_constant_ranges(std::slice::from_ref(&push_range));
        let pipeline_layout = device.create_pipeline_layout(&pl_info, None).unwrap();

        // Shaders
        let vert_code = include_bytes!("shaders/egui.vert.spv");
        let frag_code = include_bytes!("shaders/egui.frag.spv");

        let vert_words: Vec<u32> = vert_code
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let frag_words: Vec<u32> = frag_code
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let vert_module = device
            .create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&vert_words),
                None,
            )
            .unwrap();
        let frag_module = device
            .create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&frag_words),
                None,
            )
            .unwrap();

        let entry = c"main";
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(entry),
        ];

        // Vertex input: pos (vec2), uv (vec2), color (uint)
        let binding_desc = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(20) // 2*4 + 2*4 + 4
            .input_rate(vk::VertexInputRate::VERTEX);

        let attr_descs = [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .location(2)
                .binding(0)
                .format(vk::Format::R32_UINT)
                .offset(16),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_desc))
            .vertex_attribute_descriptions(&attr_descs);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        let viewport = vk::Viewport::default()
            .width(extent.width as f32)
            .height(extent.height as f32)
            .max_depth(1.0);
        let scissor = vk::Rect2D::default().extent(extent);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Pre-multiplied alpha blending
        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let pipeline = device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .unwrap()[0];

        device.destroy_shader_module(vert_module, None);
        device.destroy_shader_module(frag_module, None);

        // Descriptor pool (sized for multiple textures: font + user textures)
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(64);
        let dp_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(64)
            .pool_sizes(std::slice::from_ref(&pool_size));
        let descriptor_pool = device.create_descriptor_pool(&dp_info, None).unwrap();

        // Sampler
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = device.create_sampler(&sampler_info, None).unwrap();

        // Command pool
        let cp_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(ds.queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = device.create_command_pool(&cp_info, None).unwrap();

        // Per-image resources
        let count = images.len();
        let mut image_views = Vec::with_capacity(count);
        let mut framebuffers = Vec::with_capacity(count);
        for &image in images {
            let iv_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1),
                );
            let view = device.create_image_view(&iv_info, None).unwrap();
            image_views.push(view);

            let fb_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(std::slice::from_ref(&view))
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            framebuffers.push(device.create_framebuffer(&fb_info, None).unwrap());
        }

        let cb_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count as u32);
        let command_buffers = device.allocate_command_buffers(&cb_info).unwrap();

        let mut fences = Vec::with_capacity(count);
        let mut semaphores = Vec::with_capacity(count);
        for _ in 0..count {
            let fence = device
                .create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
                .unwrap();
            fences.push(fence);
            let sem = device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap();
            semaphores.push(sem);
        }

        Self {
            render_pass,
            pipeline_layout,
            pipeline,
            descriptor_set_layout,
            descriptor_pool,
            sampler,
            command_pool,
            textures: HashMap::new(),
            framebuffers,
            image_views,
            command_buffers,
            fences,
            semaphores,
            extent,
            format,
            vertex_buffers: (0..count).map(|_| None).collect(),
            index_buffers: (0..count).map(|_| None).collect(),
            ctx: egui::Context::default(),
            pointer_down: [false; 2],
            keys_down: [false; input::KEY_COUNT],
        }
    }

    unsafe fn destroy(&mut self, device: &ash::Device) {
        let _ = device.device_wait_idle();
        for f in &self.fences {
            device.destroy_fence(*f, None);
        }
        for s in &self.semaphores {
            device.destroy_semaphore(*s, None);
        }
        for fb in &self.framebuffers {
            device.destroy_framebuffer(*fb, None);
        }
        for iv in &self.image_views {
            device.destroy_image_view(*iv, None);
        }
        for vb in self.vertex_buffers.iter().flatten() {
            device.destroy_buffer(vb.buffer, None);
            device.free_memory(vb.memory, None);
        }
        for ib in self.index_buffers.iter().flatten() {
            device.destroy_buffer(ib.buffer, None);
            device.free_memory(ib.memory, None);
        }
        for mt in self.textures.values() {
            device.destroy_image_view(mt.vk_tex.view, None);
            device.destroy_image(mt.vk_tex.image, None);
            device.free_memory(mt.vk_tex.memory, None);
        }
        device.destroy_command_pool(self.command_pool, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        device.destroy_render_pass(self.render_pass, None);
        device.destroy_sampler(self.sampler, None);
    }

    unsafe fn update_texture(
        &mut self,
        ds: &DeviceState,
        tex_id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        let [dw, dh] = delta.image.size();
        let pixels: Vec<u8> = match &delta.image {
            egui::epaint::ImageData::Color(image) => {
                image.pixels.iter().flat_map(|p| p.to_array()).collect()
            }
        };

        match delta.pos {
            None => {
                // Full replacement — (re)create the Vulkan image.
                if let Some(old) = self.textures.remove(&tex_id) {
                    ds.device.destroy_image_view(old.vk_tex.view, None);
                    ds.device.destroy_image(old.vk_tex.image, None);
                    ds.device.free_memory(old.vk_tex.memory, None);
                    ds.device
                        .free_descriptor_sets(self.descriptor_pool, &[old.descriptor_set])
                        .ok();
                }
                let mt = self.create_texture_image(ds, dw, dh, &pixels);
                self.textures.insert(tex_id, mt);
            }
            Some([x, y]) => {
                // Partial update — merge into atlas and re-upload.
                if let Some(mt) = self.textures.get_mut(&tex_id) {
                    let full_w = mt.size[0];
                    for row in 0..dh {
                        let dst = ((y + row) * full_w + x) * 4;
                        let src = row * dw * 4;
                        if dst + dw * 4 <= mt.atlas.len() {
                            mt.atlas[dst..dst + dw * 4]
                                .copy_from_slice(&pixels[src..src + dw * 4]);
                        }
                    }
                    Self::upload_texture_data(&ds.device, &mt.vk_tex, mt.size[0], mt.size[1], &mt.atlas);
                }
            }
        }
    }

    unsafe fn free_texture(&mut self, ds: &DeviceState, tex_id: egui::TextureId) {
        if let Some(mt) = self.textures.remove(&tex_id) {
            ds.device.destroy_image_view(mt.vk_tex.view, None);
            ds.device.destroy_image(mt.vk_tex.image, None);
            ds.device.free_memory(mt.vk_tex.memory, None);
            ds.device
                .free_descriptor_sets(self.descriptor_pool, &[mt.descriptor_set])
                .ok();
        }
    }

    unsafe fn create_texture_image(
        &mut self,
        ds: &DeviceState,
        w: usize,
        h: usize,
        pixels: &[u8],
    ) -> ManagedTexture {
        let device = &ds.device;

        let img_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: w as u32,
                height: h as u32,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::LINEAR)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .initial_layout(vk::ImageLayout::PREINITIALIZED);
        let image = device.create_image(&img_info, None).unwrap();

        let mem_req = device.get_image_memory_requirements(image);
        let mem_props = ds
            .instance
            .get_physical_device_memory_properties(ds.physical_device);
        let mem_type = find_memory_type(
            &mem_props,
            mem_req.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_req.size)
            .memory_type_index(mem_type);
        let memory = device.allocate_memory(&alloc_info, None).unwrap();
        device.bind_image_memory(image, memory, 0).unwrap();

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        let view = device.create_image_view(&view_info, None).unwrap();

        let vk_tex = VkTexture {
            image,
            view,
            memory,
        };

        // Allocate a descriptor set for this texture
        let layouts = [self.descriptor_set_layout];
        let ds_alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);
        let descriptor_set = device.allocate_descriptor_sets(&ds_alloc).unwrap()[0];

        let img_info_ds = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&img_info_ds));
        device.update_descriptor_sets(&[write], &[]);

        // Upload pixel data
        Self::upload_texture_data(device, &vk_tex, w, h, pixels);

        // Transition to SHADER_READ_ONLY_OPTIMAL
        let cb = self.command_buffers[0];
        let _ = device.wait_for_fences(&[self.fences[0]], true, u64::MAX);
        let _ = device.reset_fences(&[self.fences[0]]);
        device
            .begin_command_buffer(
                cb,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();

        let barrier = vk::ImageMemoryBarrier::default()
            .image(image)
            .old_layout(vk::ImageLayout::PREINITIALIZED)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_access_mask(vk::AccessFlags::HOST_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .level_count(1)
                    .layer_count(1),
            );
        device.cmd_pipeline_barrier(
            cb,
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
        device.end_command_buffer(cb).unwrap();
        let cbs = [cb];
        let submit = vk::SubmitInfo::default().command_buffers(&cbs);
        device
            .queue_submit(ds.queue, &[submit], self.fences[0])
            .unwrap();
        let _ = device.wait_for_fences(&[self.fences[0]], true, u64::MAX);

        ManagedTexture {
            vk_tex,
            descriptor_set,
            atlas: pixels.to_vec(),
            size: [w, h],
        }
    }

    unsafe fn upload_texture_data(
        device: &ash::Device,
        tex: &VkTexture,
        width: usize,
        height: usize,
        data: &[u8],
    ) {
        let subres = vk::ImageSubresource::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR);
        let layout = device.get_image_subresource_layout(tex.image, subres);
        let row_pitch = layout.row_pitch as usize;
        let src_stride = width * 4;

        let data_ptr = device
            .map_memory(tex.memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
            .unwrap();
        let dst = data_ptr as *mut u8;

        if row_pitch == src_stride {
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        } else {
            for row in 0..height {
                let src_off = row * src_stride;
                let dst_off = layout.offset as usize + row * row_pitch;
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(src_off),
                    dst.add(dst_off),
                    src_stride,
                );
            }
        }
        device.unmap_memory(tex.memory);
    }

    unsafe fn ensure_buffer(
        device: &ash::Device,
        instance: &ash::Instance,
        phys: vk::PhysicalDevice,
        existing: &mut Option<VkBuffer>,
        usage: vk::BufferUsageFlags,
        data: &[u8],
    ) {
        let needed = data.len() as vk::DeviceSize;
        if existing.as_ref().is_some_and(|b| b.size >= needed) {
            let buf = existing.as_ref().unwrap();
            let ptr = device
                .map_memory(buf.memory, 0, needed, vk::MemoryMapFlags::empty())
                .unwrap();
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            device.unmap_memory(buf.memory);
            return;
        }

        if let Some(old) = existing.take() {
            device.destroy_buffer(old.buffer, None);
            device.free_memory(old.memory, None);
        }

        let size = needed.max(4096);
        let buf_info = vk::BufferCreateInfo::default().size(size).usage(usage);
        let buffer = device.create_buffer(&buf_info, None).unwrap();
        let mem_req = device.get_buffer_memory_requirements(buffer);
        let mem_props = instance.get_physical_device_memory_properties(phys);
        let mem_type = find_memory_type(
            &mem_props,
            mem_req.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_req.size)
            .memory_type_index(mem_type);
        let memory = device.allocate_memory(&alloc, None).unwrap();
        device.bind_buffer_memory(buffer, memory, 0).unwrap();

        let ptr = device
            .map_memory(memory, 0, needed, vk::MemoryMapFlags::empty())
            .unwrap();
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        device.unmap_memory(memory);

        *existing = Some(VkBuffer {
            buffer,
            memory,
            size,
        });
    }

    unsafe fn render_frame(
        &mut self,
        ds: &DeviceState,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore],
    ) -> vk::Semaphore {
        let draw = match crate::get_draw() {
            Some(d) => d,
            None => return vk::Semaphore::null(),
        };

        let device = &ds.device;
        let idx = image_index as usize;

        let scale = crate::ui_scale();
        let inv = 1.0 / scale;
        let (pw, ph) = (self.extent.width as f32 * inv, self.extent.height as f32 * inv);

        self.ctx.set_pixels_per_point(scale);
        let raw_input = input::gather(pw, ph, inv, &mut self.pointer_down, &mut self.keys_down);
        let output = self.ctx.run_pass(raw_input, |ctx| draw(ctx));

        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                self.update_texture(ds, *id, delta);
            }
        }
        for id in &output.textures_delta.free {
            self.free_texture(ds, *id);
        }

        let primitives = self.ctx.tessellate(output.shapes, scale);

        // Flatten egui meshes into vertex/index arrays
        let mut vertices: Vec<u8> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        struct DrawCmd {
            index_offset: u32,
            index_count: u32,
            clip_rect: egui::Rect,
            texture_id: egui::TextureId,
        }
        let mut draw_cmds = Vec::new();

        for prim in &primitives {
            let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive else {
                continue;
            };
            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                continue;
            }

            let vertex_offset = (vertices.len() / 20) as u32;
            let index_offset = indices.len() as u32;

            for v in &mesh.vertices {
                vertices.extend_from_slice(&v.pos.x.to_le_bytes());
                vertices.extend_from_slice(&v.pos.y.to_le_bytes());
                vertices.extend_from_slice(&v.uv.x.to_le_bytes());
                vertices.extend_from_slice(&v.uv.y.to_le_bytes());
                vertices.extend_from_slice(&v.color.to_array());
            }
            for &i in &mesh.indices {
                indices.push(i + vertex_offset);
            }

            draw_cmds.push(DrawCmd {
                index_offset,
                index_count: mesh.indices.len() as u32,
                clip_rect: prim.clip_rect,
                texture_id: mesh.texture_id,
            });
        }

        if draw_cmds.is_empty() {
            return vk::Semaphore::null();
        }

        let vert_bytes = unsafe {
            std::slice::from_raw_parts(vertices.as_ptr(), vertices.len())
        };
        let idx_bytes = unsafe {
            std::slice::from_raw_parts(
                indices.as_ptr() as *const u8,
                indices.len() * 4,
            )
        };

        Self::ensure_buffer(
            device,
            &ds.instance,
            ds.physical_device,
            &mut self.vertex_buffers[idx],
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vert_bytes,
        );
        Self::ensure_buffer(
            device,
            &ds.instance,
            ds.physical_device,
            &mut self.index_buffers[idx],
            vk::BufferUsageFlags::INDEX_BUFFER,
            idx_bytes,
        );

        let cb = self.command_buffers[idx];
        device
            .begin_command_buffer(
                cb,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();

        let clear_values = [];
        let rp_begin = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[idx])
            .render_area(vk::Rect2D::default().extent(self.extent))
            .clear_values(&clear_values);
        device.cmd_begin_render_pass(cb, &rp_begin, vk::SubpassContents::INLINE);
        device.cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        let screen_size = [pw, ph];
        device.cmd_push_constants(
            cb,
            self.pipeline_layout,
            vk::ShaderStageFlags::VERTEX,
            0,
            std::slice::from_raw_parts(screen_size.as_ptr() as *const u8, 8),
        );

        let viewport = vk::Viewport::default()
            .width(self.extent.width as f32)
            .height(self.extent.height as f32)
            .max_depth(1.0);
        device.cmd_set_viewport(cb, 0, &[viewport]);

        let vb = self.vertex_buffers[idx].as_ref().unwrap().buffer;
        let ib = self.index_buffers[idx].as_ref().unwrap().buffer;
        device.cmd_bind_vertex_buffers(cb, 0, &[vb], &[0]);
        device.cmd_bind_index_buffer(cb, ib, 0, vk::IndexType::UINT32);

        for cmd in &draw_cmds {
            let Some(mt) = self.textures.get(&cmd.texture_id) else {
                continue;
            };
            device.cmd_bind_descriptor_sets(
                cb,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[mt.descriptor_set],
                &[],
            );

            let min_x = (cmd.clip_rect.min.x * scale).round().max(0.0) as i32;
            let min_y = (cmd.clip_rect.min.y * scale).round().max(0.0) as i32;
            let max_x = (cmd.clip_rect.max.x * scale)
                .round()
                .min(self.extent.width as f32) as i32;
            let max_y = (cmd.clip_rect.max.y * scale)
                .round()
                .min(self.extent.height as f32) as i32;
            if max_x <= min_x || max_y <= min_y {
                continue;
            }
            device.cmd_set_scissor(
                cb,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: min_x,
                        y: min_y,
                    },
                    extent: vk::Extent2D {
                        width: (max_x - min_x) as u32,
                        height: (max_y - min_y) as u32,
                    },
                }],
            );
            device.cmd_draw_indexed(cb, cmd.index_count, 1, cmd.index_offset, 0, 0);
        }

        device.cmd_end_render_pass(cb);
        device.end_command_buffer(cb).unwrap();

        // Wait for any previous submit on this index to finish before reusing its fence
        let _ = device.wait_for_fences(&[self.fences[idx]], true, u64::MAX);
        let _ = device.reset_fences(&[self.fences[idx]]);

        let wait_stages: Vec<_> = wait_semaphores
            .iter()
            .map(|_| vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .collect();
        let signal = self.semaphores[idx];
        let cbs = [cb];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&cbs)
            .signal_semaphores(std::slice::from_ref(&signal));
        device
            .queue_submit(ds.queue, &[submit], self.fences[idx])
            .unwrap();

        signal
    }
}

// --- Vulkan hook entry points ---

#[no_mangle]
pub unsafe extern "C" fn vkGetInstanceProcAddr(
    instance: vk::Instance,
    name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    let name_str = CStr::from_ptr(name);
    match name_str.to_bytes() {
        b"vkGetInstanceProcAddr" => {
            return std::mem::transmute(vkGetInstanceProcAddr as *const c_void);
        }
        b"vkCreateDevice" => {
            return std::mem::transmute(hooked_create_device as *const c_void);
        }
        b"vkGetDeviceProcAddr" => {
            return std::mem::transmute(vkGetDeviceProcAddr as *const c_void);
        }
        _ => {}
    }

    match name_str.to_bytes() {
        b"vkQueuePresentKHR" => {
            return std::mem::transmute(hooked_queue_present as *const c_void);
        }
        b"vkCreateSwapchainKHR" => {
            return std::mem::transmute(hooked_create_swapchain as *const c_void);
        }
        b"vkDestroySwapchainKHR" => {
            return std::mem::transmute(hooked_destroy_swapchain as *const c_void);
        }
        _ => {}
    }

    let Some(real) = get_real_get_instance_proc() else {
        return None;
    };
    real(instance, name)
}

unsafe extern "C" fn hooked_create_device(
    physical_device: vk::PhysicalDevice,
    create_info: *const vk::DeviceCreateInfo,
    allocator: *const vk::AllocationCallbacks,
    device: *mut vk::Device,
) -> vk::Result {
    let real_get = get_real_get_instance_proc()
        .expect("real vkGetInstanceProcAddr must be resolved before vkCreateDevice");

    let real_create_ptr = vk_dlsym(b"vkCreateDevice\0".as_ptr() as _);
    let real_create: vk::PFN_vkCreateDevice = std::mem::transmute(real_create_ptr);
    let result = real_create(physical_device, create_info, allocator, device);
    if result != vk::Result::SUCCESS {
        return result;
    }

    let vk_device = *device;
    tracing::info!("hao-egui-overlay: Vulkan device created, installing hooks");

    // Build ash wrappers from the real function pointers
    let instance = ash::Instance::load_with(
        |name| {
            let sym = vk_dlsym(name.as_ptr());
            if !sym.is_null() {
                return sym;
            }
            std::mem::transmute(real_get(vk::Instance::null(), name.as_ptr() as _))
        },
        vk::Instance::null(),
    );

    let get_device_proc: vk::PFN_vkGetDeviceProcAddr = std::mem::transmute(
        vk_dlsym(b"vkGetDeviceProcAddr\0".as_ptr() as _),
    );
    *REAL_GET_DEVICE_PROC.lock().unwrap() = Some(get_device_proc);
    let ash_device = ash::Device::load_with(
        |name| std::mem::transmute(get_device_proc(vk_device, name.as_ptr())),
        vk_device,
    );

    // Find a graphics queue (take queue 0 of family 0 for now — good enough
    // for single-queue games, which is the common case)
    let queue_props = instance.get_physical_device_queue_family_properties(physical_device);
    let queue_family = queue_props
        .iter()
        .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .unwrap_or(0) as u32;
    let queue = ash_device.get_device_queue(queue_family, 0);

    let swapchain_fn = ash::khr::swapchain::Device::new(&instance, &ash_device);

    // Get real device-level function pointers via the real vkGetDeviceProcAddr,
    // NOT vk_dlsym — vk_dlsym returns the loader's dispatch which loops back
    // through our exported vkGetDeviceProcAddr.
    let real_queue_present: vk::PFN_vkQueuePresentKHR = std::mem::transmute(
        get_device_proc(vk_device, b"vkQueuePresentKHR\0".as_ptr() as _),
    );
    let real_create_swapchain: vk::PFN_vkCreateSwapchainKHR = std::mem::transmute(
        get_device_proc(vk_device, b"vkCreateSwapchainKHR\0".as_ptr() as _),
    );
    let real_destroy_swapchain: vk::PFN_vkDestroySwapchainKHR = std::mem::transmute(
        get_device_proc(vk_device, b"vkDestroySwapchainKHR\0".as_ptr() as _),
    );

    *DEVICE_MAP.lock().unwrap() = Some(DeviceState {
        instance,
        device: ash_device,
        physical_device,
        queue,
        queue_family,
        swapchain_fn,
        real_queue_present,
        real_create_swapchain,
        real_destroy_swapchain,
    });

    result
}

#[cfg(feature = "hooking")]
pub(crate) fn vk_get_device_proc_addr_raw() -> *const () {
    vkGetDeviceProcAddr as *const ()
}

fn get_real_get_device_proc() -> Option<vk::PFN_vkGetDeviceProcAddr> {
    let mut guard = REAL_GET_DEVICE_PROC.lock().unwrap();
    if let Some(f) = *guard {
        return Some(f);
    }
    let sym = unsafe { vk_dlsym(b"vkGetDeviceProcAddr\0".as_ptr() as _) };
    if sym.is_null() {
        return None;
    }
    let f = unsafe { std::mem::transmute(sym) };
    *guard = Some(f);
    Some(f)
}

#[no_mangle]
pub unsafe extern "C" fn vkGetDeviceProcAddr(
    device: vk::Device,
    name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    let name_str = CStr::from_ptr(name);
    match name_str.to_bytes() {
        b"vkQueuePresentKHR" => {
            return std::mem::transmute(hooked_queue_present as *const c_void);
        }
        b"vkCreateSwapchainKHR" => {
            return std::mem::transmute(hooked_create_swapchain as *const c_void);
        }
        b"vkDestroySwapchainKHR" => {
            return std::mem::transmute(hooked_destroy_swapchain as *const c_void);
        }
        b"vkGetDeviceProcAddr" => {
            return std::mem::transmute(vkGetDeviceProcAddr as *const c_void);
        }
        _ => {}
    }
    match get_real_get_device_proc() {
        Some(real) => real(device, name),
        None => {
            let sym = vk_dlsym(name);
            std::mem::transmute(sym)
        }
    }
}

unsafe extern "C" fn hooked_create_swapchain(
    device: vk::Device,
    create_info: *const vk::SwapchainCreateInfoKHR,
    allocator: *const vk::AllocationCallbacks,
    swapchain: *mut vk::SwapchainKHR,
) -> vk::Result {
    let ds_guard = DEVICE_MAP.lock().unwrap();
    let ds = ds_guard.as_ref().unwrap();
    let result = (ds.real_create_swapchain)(device, create_info, allocator, swapchain);
    if result != vk::Result::SUCCESS {
        return result;
    }

    let info = &*create_info;
    let format = info.image_format;
    let extent = info.image_extent;

    tracing::info!(
        ?format,
        w = extent.width,
        h = extent.height,
        "hao-egui-overlay: Vulkan swapchain created"
    );

    let images = ds
        .swapchain_fn
        .get_swapchain_images(*swapchain)
        .unwrap();

    // Destroy old overlay if any
    let mut state = STATE.lock().unwrap();
    if let Some(mut old) = state.take() {
        old.destroy(&ds.device);
    }
    *state = Some(VkOverlay::new(ds, &images, format, extent));
    drop(state);
    drop(ds_guard);

    result
}

unsafe extern "C" fn hooked_destroy_swapchain(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    allocator: *const vk::AllocationCallbacks,
) {
    {
        let ds_guard = DEVICE_MAP.lock().unwrap();
        let ds = ds_guard.as_ref().unwrap();
        let mut state = STATE.lock().unwrap();
        if let Some(mut overlay) = state.take() {
            overlay.destroy(&ds.device);
        }
        (ds.real_destroy_swapchain)(device, swapchain, allocator);
    }
}

unsafe extern "C" fn hooked_queue_present(
    queue: vk::Queue,
    present_info: *const vk::PresentInfoKHR,
) -> vk::Result {
    let ds_guard = DEVICE_MAP.lock().unwrap();
    let ds = ds_guard.as_ref().unwrap();
    let info = &*present_info;

    let wait_sems = if info.wait_semaphore_count > 0 {
        std::slice::from_raw_parts(info.p_wait_semaphores, info.wait_semaphore_count as usize)
    } else {
        &[]
    };
    let image_indices =
        std::slice::from_raw_parts(info.p_image_indices, info.swapchain_count as usize);

    let mut state = STATE.lock().unwrap();
    let mut our_sem = vk::Semaphore::null();
    if let Some(overlay) = state.as_mut() {
        if info.swapchain_count > 0 {
            our_sem = overlay.render_frame(ds, image_indices[0], wait_sems);
        }
    }
    drop(state);

    if our_sem != vk::Semaphore::null() {
        let mut modified = *info;
        modified.wait_semaphore_count = 1;
        modified.p_wait_semaphores = &our_sem;
        let result = (ds.real_queue_present)(queue, &modified);
        drop(ds_guard);
        return result;
    }

    let result = (ds.real_queue_present)(queue, present_info);
    drop(ds_guard);
    result
}

pub mod hooks {
    // vkGetInstanceProcAddr is exported from the parent module.
}
