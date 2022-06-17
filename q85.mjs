const i2c=[...new TextEncoder().encode('!&()*+,-.0123456789:;=?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~')];
const c2i=[];
i2c.forEach((it,i)=>c2i[it-33]=i);
const five=[0,1,2,3,4];
function* chunks(arr, n) {
  if(n===0) yield arr;
  else for(let i=0; i<arr.length; i+=n) yield arr.slice(i,i+n);
}
const buffer=data=>{
  if(typeof data==='string') data=new TextEncoder().encode(data);
  if(ArrayBuffer.isView(data)) data=data.buffer;
  if(data instanceof ArrayBuffer) return data;
  throw new Error('Expected string, typed array or array buffer.');
}
const enc85=data=>{
  let buf=buffer(data);
  if(buf.byteLength%4!==0){
    const enlarged=new Uint8Array(Math.ceil(buf.byteLength/4)*4);
    enlarged.set(new Uint8Array(buf));
    buf=enlarged.buffer;
  }
  const res=[...new Uint32Array(buf)].flatMap(it=>five.map(_=>{
    const c=i2c[it%85];
    it=Math.trunc(it/85);
    return c;
  }));
  return new TextDecoder().decode(Uint8Array.from(res)).replace(/!+$/,'');
};
const dec85=data=>{
  const buf=buffer(data);
  const u32s=[...chunks(new Uint8Array(buf),5)].map(it=>it.reverse().reduce((prev,it)=>prev*85+c2i[it-33],0));
  return new TextDecoder().decode(new Uint32Array(u32s)).replace(/\u0000+$/,'');
}
export {enc85,dec85};
