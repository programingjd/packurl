import {assertEquals} from "https://deno.land/std/testing/asserts.ts";
import {dec85,enc85} from '../q85.mjs';

Deno.test("Encode/Decode ascii",()=>{
  const text='Testing data.';
  const encoded=enc85(text);
  assertEquals(encoded,'v&CMN1[;L1pyFDHW');
  const decoded=dec85(encoded);
  assertEquals(decoded,text);
});
Deno.test("Encode/Decode unicode",()=>{
  const text='てスト 🤔';
  const encoded=enc85(text);
  assertEquals(encoded,'oK45sxT,JSc}DK]c?+');
  const decoded=dec85(encoded);
  assertEquals(decoded,text);
});
Deno.test("Encode/Decode binary",()=>{
  const text='!4 てスト 🤔';
  const encoded=enc85(text);
  assertEquals(encoded,'MVfD&oK45sxT,JSc}DK]fQI');
  const decoded=dec85(encoded);
  assertEquals(decoded,text);
});

