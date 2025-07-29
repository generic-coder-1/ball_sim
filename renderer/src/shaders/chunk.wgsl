const CHUNK_SIZE: u32 = 32;

struct ChunkInstance {
    position: vec2<i32>,
    tileIndices: array<u32, CHUNK_SIZE * CHUNK_SIZE / 4>, //we divide by 4 because we only need one byte to index into the tile atlas
};

struct VertexInput {
  @location(0) position: vec2<f32>, // local vertex position of quad
  @builtin(instance_index) index: u32,
};

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) index: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput{
  let chunk = chunkInstances[input.index];
  
  let world_pos = input.position + vec2<f32>(chunk.position);
  let scale = min(camera.screensize.x, camera.screensize.y*camera.min_ratio)/camera.width;
    
  let camera_relative_pos = ((world_pos-camera.pos)*scale/camera.screensize)*camera.screensize;
  let ndc = camera_relative_pos/camera.screensize*2.0;

  var out: VertexOutput;
  out.uv = input.position; 
  out.uv.y = 1.0 - out.uv.y;
  out.position = vec4<f32>(ndc, 0.0, 1.0);
  out.index = input.index;
  return out;
}

struct AtlasInfo {
  tilesPerRow: u32,
  _pad: u32,
  tileSize: vec2<u32>,  // e.g., vec2(8.0) for 8x8 tiles
};

struct Camera{
  pos: vec2<f32>,
  screensize: vec2<f32>,
  width:f32,
  min_ratio: f32,
}

@group(0) @binding(0) var<storage, read> chunkInstances: array<ChunkInstance>;

@group(1) @binding(0) var atlasTex: texture_2d<f32>;
@group(1) @binding(1) var atlasSampler: sampler;
@group(1) @binding(2) var<uniform> atlasInfo: AtlasInfo;

@group(2) @binding(0) var<uniform> camera: Camera;

@fragment
fn fs_main(
  @location(0) uv: vec2<f32>,
  @location(1) instanceIndex: u32
) -> @location(0) vec4<f32> {
  let chunk = chunkInstances[instanceIndex];

  // Determine which tile in chunk UV hits
  let tileUV = uv * vec2<f32>(f32(CHUNK_SIZE));
  let tileCoord = min(vec2<u32>(tileUV), vec2(CHUNK_SIZE - 1));
  let tileIndexInChunk = tileCoord.y * CHUNK_SIZE + tileCoord.x;

  // Lookup tile index from chunk
  var tileIndex = chunk.tileIndices[tileIndexInChunk/4u];
  tileIndex = (tileIndex >> (tileIndexInChunk%4u * 8u)) & 0xFFu; 

  let current_pixel = min(vec2<u32>(uv * vec2<f32>(CHUNK_SIZE) * f32(atlasInfo.tileSize.x)), (atlasInfo.tileSize * vec2(CHUNK_SIZE)) - vec2(1));

  let tile_col = tileIndex % atlasInfo.tilesPerRow;
  let tile_row = tileIndex / atlasInfo.tilesPerRow;
  let atlas_tile_offset = vec2<u32>(tile_col, tile_row)*atlasInfo.tileSize;
  let atlas_uv:vec2<u32> = min(atlas_tile_offset + current_pixel%atlasInfo.tileSize, textureDimensions(atlasTex) - vec2(1));

  let color = textureLoad(atlasTex, atlas_uv, 0);
  if color.w<0.999{
    discard;
  }
  return color;
}
