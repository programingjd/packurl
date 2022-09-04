const i2c=[...new TextEncoder().encode('!&()*+,-.0123456789:;=?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~')];
const c2i=[];
i2c.forEach((it,i)=>c2i[it-33]=i);
/**
 * @typedef {Int8Array|Uint8Array|Uint8ClampedArray|Int16Array|Uint16Array|Int32Array|
 *           Uint32Array|Float32Array|Float64Array|BigInt64Array|BigUint64Array} TypedArray
 */
/**
 * @template T
 * @template {Array<T>|TypedArray} A
 * @param {A} arr
 * @param {number} n
 * @return {Generator<A,void,*>}
 */
function* chunks(arr, n) {
  if(n===0) yield arr;
  else for(let i=0; i<arr.length; i+=n) yield arr.slice(i,i+n);
}
/**
 * @param {string|TypedArray} data
 * @return {Uint8Array}
 */
const asBytes=data=>{
  if(typeof data==='string') data=new TextEncoder().encode(data);
  if(data instanceof Uint8Array) return data;
  if(ArrayBuffer.isView(data)) return new Uint8Array(data.buffer,0,data.byteLength);
  throw new Error('Expected string or typed array.');
}
/**
 * @param {string|TypedArray} data
 * @return {string}
 */
const enc85=data=>{
  const bytes=asBytes(data);
  const suffixed=new Uint8Array(bytes.byteLength+1);
  suffixed.set(bytes);
  suffixed.set([1],bytes.byteLength);
  /** @type {Array<number>} */
  const chars=[...chunks(suffixed,4)].flatMap(chunk=>{
    let u32=chunk.reverse().reduce((prev,it)=>prev*256+it,0);
    const chars=[];
    for(let i=0;i<5;++i){
      chars.push(i2c[u32%85]);
      u32=Math.trunc(u32/85);
    }
    if(chunk.length===4) return chars;
    for(let i=0;i<5;++i){
      if(chars[chars.length-1]===33) chars.pop();
      else break;
    }
    return chars;
  });
  return new TextDecoder().decode(Uint8Array.from(chars));
}
/**
 * @param {string} data
 * @return {Uint8Array}
 */
const dec85=data=>{
  /** @type {Array<number>} */
  const bytes=[...chunks(asBytes(data),5)].flatMap(chunk=>{
    let u32=chunk.reverse().reduce((prev,it)=>prev*85+c2i[it-33],0);
    const bytes=[];
    for(let i=0;i<4;++i){
      bytes.push(u32%256);
      u32=Math.trunc(u32/256);
    }
    if(chunk.length===5) return bytes;
    for(let i=0;i<4;++i){
      if(bytes[bytes.length-1]===0) bytes.pop();
      else break;
    }
    return bytes;
  });
  bytes.pop();
  return Uint8Array.from(bytes);
}

export {enc85,dec85};
