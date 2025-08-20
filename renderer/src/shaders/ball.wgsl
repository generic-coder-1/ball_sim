const BALL_SIZE: u32 = 16;

struct VertexInput {
  @location(0) position: vec2<f32>, // local vertex position of quad
  @builtin(instance_index) index: u32,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) on: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput{
  let ball_pos = ballPositionInstance[input.index];
  let ball_on = ballOnInstance[input.index];
  
  let world_pos = input.position + vec2<f32>(ball_pos.pos);
  let scale = min(camera.screensize.x, camera.screensize.y*camera.min_ratio)/camera.width;
    
  let camera_relative_pos = ((world_pos-camera.pos)*scale/camera.screensize)*camera.screensize;
  let ndc = camera_relative_pos/camera.screensize*2.0;

  var out: VertexOutput;
  out.uv = input.position; 
  out.uv.y = 1.0 - out.uv.y;
  out.position = vec4<f32>(ndc, 0.0, 1.0);
  out.on = ball_on;
  return out;
}

struct Camera{
  pos: vec2<f32>,
  screensize: vec2<f32>,
  width:f32,
  min_ratio: f32,
}

struct BallInstance{
  pos: vec2<i32>,
}

@group(0) @binding(0) var<storage, read> ballPositionInstance: array<BallInstance>;
@group(0) @binding(1) var<storage, read> ballOnInstance: array<u32>;

@group(1) @binding(0) var ball_tex: texture_2d<f32>;

@group(2) @binding(0) var<uniform> camera: Camera;

@fragment
fn fs_main(
  @location(0) uv: vec2<f32>,
  @location(1) on: u32,
) -> @location(0) vec4<f32> {
  var current_pixel = vec2<u32>(uv * f32(BALL_SIZE));
  if on != 1{
    current_pixel.x += BALL_SIZE; 
  }
  let color = textureLoad(ball_tex, current_pixel, 0);
  if color.w<0.999{
    discard;
  }
  return color;
}
