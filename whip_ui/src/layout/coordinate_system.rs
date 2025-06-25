use bevy_math::Vec3;
use bevy_transform::prelude::Transform;

/// TOML file coordinates (top-left origin, Y increases downward)
/// 
/// These coordinates come directly from TOML layout files where:
/// - Origin (0,0) is at the top-left corner of the window
/// - X increases rightward  
/// - Y increases downward
/// 
/// Use .to_bevy() to convert for Transform components
/// Use .to_taffy() to convert for Taffy layout calculations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TomlCoords(pub Vec3);

/// Bevy Transform coordinates (bottom-left origin, Y increases upward)
/// 
/// These coordinates are used by Bevy's Transform component where:
/// - Origin (0,0) is at the bottom-left corner of the window
/// - X increases rightward
/// - Y increases upward
/// 
/// This is the coordinate system used for final entity positioning
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevyCoords(pub Vec3);

/// Taffy layout coordinates (top-left origin, Y increases downward)
/// 
/// These coordinates are used by the Taffy layout engine where:
/// - Origin (0,0) is at the top-left corner of the container
/// - X increases rightward
/// - Y increases downward
/// 
/// Use .to_bevy() to convert for Transform components
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaffyCoords(pub Vec3);

/// Vulkan rendering coordinates (bottom-left origin, Y increases upward)
/// 
/// These coordinates are used by the Vulkan rendering pipeline where:
/// - Origin (0,0) is at the bottom-left corner of the framebuffer
/// - X increases rightward  
/// - Y increases upward
/// 
/// This matches Bevy coordinates for seamless integration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VulkanCoords(pub Vec3);

impl TomlCoords {
    /// Create new TOML coordinates
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
    
    /// Convert to Bevy Transform coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_bevy(self, window_height: f32) -> BevyCoords {
        BevyCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Convert to Taffy layout coordinates (no conversion needed)
    pub fn to_taffy(self) -> TaffyCoords {
        TaffyCoords(self.0)
    }
    
    /// Convert to Vulkan rendering coordinates (flips Y axis)
    /// 
    /// # Arguments  
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_vulkan(self, window_height: f32) -> VulkanCoords {
        VulkanCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Get the raw Vec3 value (use sparingly)
    pub fn raw(self) -> Vec3 {
        self.0
    }
}

impl BevyCoords {
    /// Create new Bevy coordinates
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
    
    /// Convert to TOML coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_toml(self, window_height: f32) -> TomlCoords {
        TomlCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Convert to Taffy layout coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_taffy(self, window_height: f32) -> TaffyCoords {
        TaffyCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Convert to Vulkan rendering coordinates (no conversion needed)
    pub fn to_vulkan(self) -> VulkanCoords {
        VulkanCoords(self.0)
    }
    
    /// Get the raw Vec3 value (use sparingly)
    pub fn raw(self) -> Vec3 {
        self.0
    }
}

impl TaffyCoords {
    /// Create new Taffy coordinates
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
    
    /// Convert to Bevy Transform coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_bevy(self, window_height: f32) -> BevyCoords {
        BevyCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Convert to TOML coordinates (no conversion needed)
    pub fn to_toml(self) -> TomlCoords {
        TomlCoords(self.0)
    }
    
    /// Convert to Vulkan rendering coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_vulkan(self, window_height: f32) -> VulkanCoords {
        VulkanCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Get the raw Vec3 value (use sparingly)
    pub fn raw(self) -> Vec3 {
        self.0
    }
}

impl VulkanCoords {
    /// Create new Vulkan coordinates
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }
    
    /// Convert to Bevy Transform coordinates (no conversion needed)
    pub fn to_bevy(self) -> BevyCoords {
        BevyCoords(self.0)
    }
    
    /// Convert to TOML coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_toml(self, window_height: f32) -> TomlCoords {
        TomlCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Convert to Taffy layout coordinates (flips Y axis)
    /// 
    /// # Arguments
    /// * `window_height` - Height of the window for Y coordinate conversion
    pub fn to_taffy(self, window_height: f32) -> TaffyCoords {
        TaffyCoords(Vec3::new(
            self.0.x,
            window_height - self.0.y, // Flip Y axis
            self.0.z,
        ))
    }
    
    /// Get the raw Vec3 value (use sparingly)
    pub fn raw(self) -> Vec3 {
        self.0
    }
}

/// Helper functions for creating UI transforms with the correct coordinate types

/// Create a Bevy Transform from UI coordinates
/// 
/// This function only accepts BevyCoords to prevent coordinate system mixing
pub fn create_ui_transform(position: BevyCoords) -> Transform {
    Transform::from_translation(position.raw())
}

/// Update an existing Transform with UI coordinates
/// 
/// This function only accepts BevyCoords to prevent coordinate system mixing
pub fn update_ui_transform(transform: &mut Transform, position: BevyCoords) {
    transform.translation = position.raw();
}

impl From<Vec3> for TomlCoords {
    fn from(vec: Vec3) -> Self {
        Self(vec)
    }
}

impl From<Vec3> for BevyCoords {
    fn from(vec: Vec3) -> Self {
        Self(vec)
    }
}

impl From<Vec3> for TaffyCoords {
    fn from(vec: Vec3) -> Self {
        Self(vec)
    }
}

impl From<Vec3> for VulkanCoords {
    fn from(vec: Vec3) -> Self {
        Self(vec)
    }
}

impl From<TomlCoords> for Vec3 {
    fn from(coords: TomlCoords) -> Self {
        coords.0
    }
}

impl From<BevyCoords> for Vec3 {
    fn from(coords: BevyCoords) -> Self {
        coords.0
    }
}

impl From<TaffyCoords> for Vec3 {
    fn from(coords: TaffyCoords) -> Self {
        coords.0
    }
}

impl From<VulkanCoords> for Vec3 {
    fn from(coords: VulkanCoords) -> Self {
        coords.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_toml_to_bevy_conversion() {
        let window_height = 600.0;
        let toml_coords = TomlCoords::new(100.0, 50.0, 1.0);
        let bevy_coords = toml_coords.to_bevy(window_height);
        
        // Y should be flipped: 600 - 50 = 550
        assert_eq!(bevy_coords.raw(), Vec3::new(100.0, 550.0, 1.0));
    }
    
    #[test]
    fn test_bevy_to_toml_conversion() {
        let window_height = 600.0;
        let bevy_coords = BevyCoords::new(100.0, 550.0, 1.0);
        let toml_coords = bevy_coords.to_toml(window_height);
        
        // Y should be flipped: 600 - 550 = 50
        assert_eq!(toml_coords.raw(), Vec3::new(100.0, 50.0, 1.0));
    }
    
    #[test]
    fn test_taffy_to_bevy_conversion() {
        let window_height = 600.0;
        let taffy_coords = TaffyCoords::new(100.0, 50.0, 1.0);
        let bevy_coords = taffy_coords.to_bevy(window_height);
        
        // Y should be flipped: 600 - 50 = 550
        assert_eq!(bevy_coords.raw(), Vec3::new(100.0, 550.0, 1.0));
    }
    
    #[test]
    fn test_coordinate_roundtrip() {
        let window_height = 600.0;
        let original = TomlCoords::new(200.0, 150.0, 2.0);
        
        // Convert through multiple coordinate systems and back
        let roundtrip = original
            .to_bevy(window_height)
            .to_taffy(window_height)
            .to_toml();
        
        assert_eq!(original, roundtrip);
    }
    
    #[test]
    fn test_create_ui_transform() {
        let bevy_coords = BevyCoords::new(100.0, 200.0, 1.0);
        let transform = create_ui_transform(bevy_coords);
        
        assert_eq!(transform.translation, Vec3::new(100.0, 200.0, 1.0));
    }
    
    // Additional comprehensive coordinate tests
    
    #[test]
    fn test_main_toml_widget_positions() {
        // Test positions from the actual main.toml file
        let window_height = 300.0; // From main.toml window config
        
        // Triangle position: [300.0, 200.0, -1.0]
        let triangle_toml = TomlCoords::new(300.0, 200.0, -1.0);
        let triangle_bevy = triangle_toml.to_bevy(window_height);
        assert_eq!(triangle_bevy.raw(), Vec3::new(300.0, 100.0, -1.0)); // 300 - 200 = 100
        
        // Square position: [125.0, 225.0, -2.0] 
        let square_toml = TomlCoords::new(125.0, 225.0, -2.0);
        let square_bevy = square_toml.to_bevy(window_height);
        assert_eq!(square_bevy.raw(), Vec3::new(125.0, 75.0, -2.0)); // 300 - 225 = 75
        
        // Text position: [50.0, 50.0, -2.0]
        let text_toml = TomlCoords::new(50.0, 50.0, -2.0);
        let text_bevy = text_toml.to_bevy(window_height);
        assert_eq!(text_bevy.raw(), Vec3::new(50.0, 250.0, -2.0)); // 300 - 50 = 250
    }
    
    #[test]
    fn test_vulkan_coordinate_compatibility() {
        let window_height = 600.0;
        let bevy_coords = BevyCoords::new(100.0, 200.0, 1.0);
        let vulkan_coords = bevy_coords.to_vulkan();
        
        // Vulkan and Bevy should use same coordinate system
        assert_eq!(vulkan_coords.raw(), bevy_coords.raw());
    }
    
    #[test]
    fn test_coordinate_type_safety() {
        // This test ensures that coordinate conversions require explicit typing
        let window_height = 600.0;
        let raw_vec = Vec3::new(100.0, 50.0, 1.0);
        
        // Test that all coordinate types can be created from Vec3
        let toml_coords = TomlCoords::from(raw_vec);
        let bevy_coords = BevyCoords::from(raw_vec);
        let taffy_coords = TaffyCoords::from(raw_vec);
        let vulkan_coords = VulkanCoords::from(raw_vec);
        
        // Test conversions between types
        let toml_to_bevy = toml_coords.to_bevy(window_height);
        let bevy_to_taffy = bevy_coords.to_taffy(window_height);
        let taffy_to_vulkan = taffy_coords.to_vulkan(window_height);
        let vulkan_to_toml = vulkan_coords.to_toml(window_height);
        
        // Verify specific conversions
        assert_eq!(toml_to_bevy.raw(), Vec3::new(100.0, 550.0, 1.0)); // Y flipped
        assert_eq!(bevy_to_taffy.raw(), Vec3::new(100.0, 550.0, 1.0)); // Y flipped  
        assert_eq!(taffy_to_vulkan.raw(), Vec3::new(100.0, 550.0, 1.0)); // Y flipped
        assert_eq!(vulkan_to_toml.raw(), Vec3::new(100.0, 550.0, 1.0)); // Y flipped
    }
    
    #[test]
    fn test_edge_cases() {
        let window_height = 300.0;
        
        // Test zero coordinates
        let zero_toml = TomlCoords::new(0.0, 0.0, 0.0);
        let zero_bevy = zero_toml.to_bevy(window_height);
        assert_eq!(zero_bevy.raw(), Vec3::new(0.0, 300.0, 0.0)); // Top-left becomes bottom-left
        
        // Test bottom coordinates in TOML (should become top in Bevy)
        let bottom_toml = TomlCoords::new(0.0, 300.0, 0.0);
        let bottom_bevy = bottom_toml.to_bevy(window_height);
        assert_eq!(bottom_bevy.raw(), Vec3::new(0.0, 0.0, 0.0)); // Bottom in TOML becomes origin in Bevy
        
        // Test negative Z coordinates (behind camera)
        let behind_toml = TomlCoords::new(100.0, 100.0, -10.0);
        let behind_bevy = behind_toml.to_bevy(window_height);
        assert_eq!(behind_bevy.raw(), Vec3::new(100.0, 200.0, -10.0)); // Z preserved, Y flipped
    }
    
    #[test]
    fn test_red_rectangle_coordinate_conversion() {
        // Test the specific coordinate conversion for the red rectangle issue
        let window_height = 300.0; // From main.toml window config
        
        // Red rectangle TOML position: [400.0, 100.0, -1.0]
        let red_rect_toml = TomlCoords::new(400.0, 100.0, -1.0);
        let red_rect_bevy = red_rect_toml.to_bevy(window_height);
        
        // Expected Bevy position: [400.0, 300.0 - 100.0, -1.0] = [400.0, 200.0, -1.0]
        assert_eq!(red_rect_bevy.raw(), Vec3::new(400.0, 200.0, -1.0));
        
        // Verify that this position is 200px from bottom, 100px from top (as intended in TOML)
        assert_eq!(red_rect_bevy.raw().y, 200.0); // 200px from bottom of 300px window
    }
    
    #[test]
    fn test_absolute_vs_relative_positioning() {
        let window_height = 300.0;
        
        // Test absolute positioning: TOML coordinates should convert directly to Bevy
        let absolute_toml = TomlCoords::new(400.0, 100.0, -1.0);
        let absolute_bevy = absolute_toml.to_bevy(window_height);
        assert_eq!(absolute_bevy.raw(), Vec3::new(400.0, 200.0, -1.0));
        
        // Test relative positioning: Taffy coordinates should convert to Bevy
        // For this test, assume Taffy computed the same position (in its coordinate space)
        let relative_taffy = TaffyCoords::new(400.0, 100.0, -1.0);
        let relative_bevy = relative_taffy.to_bevy(window_height);
        assert_eq!(relative_bevy.raw(), Vec3::new(400.0, 200.0, -1.0));
        
        // Both should yield the same result, but the conversion path is different
        assert_eq!(absolute_bevy.raw(), relative_bevy.raw());
    }
    
    #[test] 
    fn test_window_resize_coordinate_stability() {
        // Test that absolute positioned elements maintain their intended position
        // relative to the window when window height changes
        let red_rect_toml = TomlCoords::new(400.0, 100.0, -1.0);
        
        // Test with different window heights
        let small_window = 200.0;
        let normal_window = 300.0;
        let large_window = 600.0;
        
        let bevy_small = red_rect_toml.to_bevy(small_window);
        let bevy_normal = red_rect_toml.to_bevy(normal_window);
        let bevy_large = red_rect_toml.to_bevy(large_window);
        
        // X and Z should remain constant
        assert_eq!(bevy_small.raw().x, 400.0);
        assert_eq!(bevy_normal.raw().x, 400.0); 
        assert_eq!(bevy_large.raw().x, 400.0);
        assert_eq!(bevy_small.raw().z, -1.0);
        assert_eq!(bevy_normal.raw().z, -1.0);
        assert_eq!(bevy_large.raw().z, -1.0);
        
        // Y should change to maintain 100px from top
        assert_eq!(bevy_small.raw().y, 100.0);   // 200 - 100 = 100
        assert_eq!(bevy_normal.raw().y, 200.0);  // 300 - 100 = 200  
        assert_eq!(bevy_large.raw().y, 500.0);   // 600 - 100 = 500
        
        // Verify the element stays 100px from the top in each case
        assert_eq!(small_window - bevy_small.raw().y, 100.0);
        assert_eq!(normal_window - bevy_normal.raw().y, 100.0);
        assert_eq!(large_window - bevy_large.raw().y, 100.0);
    }
}