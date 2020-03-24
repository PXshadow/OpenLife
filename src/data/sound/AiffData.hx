package data.sound;

import lime.utils.Int16Array;
import openfl.utils.ByteArray;
import haxe.io.BytesData;
import lime.media.AudioSource;
import lime.media.AudioBuffer;
import lime.utils.UInt8Array;
import haxe.io.Float32Array;
import lime.media.openal.AL;
import haxe.ds.Vector;
import haxe.io.Bytes;
import openfl.media.Sound;
#if nativeGen @:nativeGen #end
class AiffData
{
    public function new(data:Bytes)
    {
        readMono16AIFFData(ByteArray.fromBytes(data));
    }
    public function readMono16AIFFData(data:ByteArray)
    {
        trace("data length " + data.length + " bit");
        if (data.length < 34)
        {
            trace("Not long enough for header");
            return;
        }
        //num channels
        if (data[20] != 0 || data[21] != 1)
        {
            trace("AIFF not mono");
            return;
        }
        if (data[26] != 0 || data[27] != 16)
        {
            trace("AIFF not 16-bit");
            return;
        }
        var numSamples = data[22] << 24 | data[23] << 16 | data[24] << 8 | data[25];
        var sampleRate = data[30] << 8 | data[31];
    
        var sampleStartByte = 54;
        var numBytes = numSamples * 2;
        if (data.length < sampleStartByte + numBytes)
        {
            trace("AIFF not long enought for Data");
            return;
        }
        var samples = new Int16Array(numSamples);
        //samples.type = lime.utils.ArrayBufferView.TypedArrayType.Float64;
        //samples.type = lime.utils.ArrayBufferView.TypedArrayType.Int16;
        //var samples = Bytes.alloc(sampleRate * 4);
        var b = sampleStartByte;
        for (i in 0... numSamples)
        {
            //samples[i] = data.bytes.getUI8(i + sampleStartByte);
            samples[i] = (data[b] << 8) | data[b + 1];
            //samples[i] = (data[b] << 8) | data[b + 1];
            //samples.setUInt16(i,(data[b] << 8) | data[b + 1]);
            //samples[b] = data[b];
            //samples[b + 1] = data[b + 1];
            b += 2;
        }
        var buffer = new AudioBuffer();
        buffer.bitsPerSample = 16;
        buffer.channels = 1;
        buffer.sampleRate = sampleRate;
        buffer.data = UInt8Array.fromBytes(samples.toBytes());
        Sound.fromAudioBuffer(buffer).play(0,999);
    }
}