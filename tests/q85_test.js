import {assertEquals} from "https://deno.land/std/testing/asserts.ts";
import {dec85 as dec85_,enc85} from '../q85.mjs';
const dec85=data=>new TextDecoder().decode(dec85_(data));


Deno.test("Encode/Decode ascii",()=>{
  const text='Testing data.';
  const encoded=enc85(text);
  assertEquals(encoded,'v&CMN1[;L1pyFDHX)');
  const decoded=dec85(encoded);
  assertEquals(decoded,text);
});
Deno.test("Encode/Decode unicode",()=>{
  const text='てスト 🤔';
  const encoded=enc85(text);
  assertEquals(encoded,'oK45sxT,JSc}DK]dE5');
  const decoded=dec85(encoded);
  assertEquals(decoded,text);
});
Deno.test("Encode/Decode binary",()=>{
  const binary=String.fromCharCode(
    0x21,0x54,0x00,0x04,0x3c,0x64,0x69,0x76,0x3e,0x54,0x65,0x73,0x74,0x20,0x64,0x61,
    0x74,0x61,0x2e,0x3c,0x2f,0x64,0x69,0x76,0x3e,0x0a,0x03
  );
  assertEquals(binary.length,27);
  const encoded=enc85(binary);
  assertEquals(encoded,'MKCA&TRt*O?DR-N+T^BHj+-F:GRt*Ovc`D!');
  const decoded=dec85(encoded);
  assertEquals(decoded,binary);
});

