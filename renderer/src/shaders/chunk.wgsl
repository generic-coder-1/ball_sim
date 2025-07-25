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
  
  let world_pos = input.position + vec2<f32>(chunk.position * i32(CHUNK_SIZE));

  var out: VertexOutput;
  out.position = vec4<f32>((world_pos - camera.position) * camera.scale, 0.0, 1.0);
  out.uv = input.position; 
  out.index = input.index;
  return out;
}

struct AtlasInfo {
  tilesPerRow: u32,
  _pad: u32,
  tileSize: vec2<f32>,  // e.g., vec2(8.0) for 8x8 tiles
};

struct Camera{
  position: vec2<f32>,
  _pad: u32,
  scale:f32,
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
  let tileCoord = vec2<u32>(tileUV);
  let tileIndexInChunk = tileCoord.y * CHUNK_SIZE + tileCoord.x;

  // Lookup tile index from chunk
  var tileIndex = chunk.tileIndices[tileIndexInChunk/4u];
  tileIndex = (tileIndex >> (tileIndexInChunk%4u * 8u)) & 0xFFu; 

  // Compute atlas UV offset
  let tileX = f32(tileIndex % atlasInfo.tilesPerRow);
  let tileY = f32(tileIndex / atlasInfo.tilesPerRow);
  let tileOffset = vec2<f32>(tileX, tileY) / atlasInfo.tileSize;

  // Compute local UV inside tile
  let tileFrac = fract(tileUV);
  let atlasUV = tileOffset + tileFrac * atlasInfo.tileSize;

  return textureSample(atlasTex, atlasSampler, atlasUV);
}
