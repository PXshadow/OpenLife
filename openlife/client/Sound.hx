import hxd.snd.ChannelGroup;
import hxd.snd.effect.Spatialization;
import hxd.snd.effect.Pitch;
import hxd.snd.Effect;
import haxe.io.UInt8Array;
import hxd.fs.FileEntry;
import hxd.snd.NativeChannel;
import hxd.snd.Data;
import hxd.snd.Manager;
import hxd.snd.Channel;
import hxd.snd.Data.SampleFormat;
import openlife.resources.Resource;
import haxe.io.Bytes;

class Sound extends hxd.snd.Data {
	var rawData:Bytes;

	public function new(id:Int) {
		readMono16AIFFData(Resource.sound(id));
		var res = new SoundRes(this);
		res.play(false, 1, Game.soundChannel);
	}

	private function readMono16AIFFData(data:Bytes) {
		if (data.length < 34) throw "Not long enough for header";
		if (data.get(20) != 0 || data.get(21) != 1) throw "aiff not mono";
		if (data.get(26) != 0 || data.get(27) != 16) throw "aiff not 16-bit";
		var numSamples = data.get(22) << 24 | data.get(23) << 16 | data.get(24) << 8 | data.get(25);
		var sampleRate = data.get(30) << 8 | data.get(31);

		var sampleStartByte = 54;
		var numBytes = numSamples * 2;
		if (data.length < sampleStartByte + numBytes) throw "AIFF not long enough for data";
		rawData = Bytes.alloc(numBytes);
		var b = sampleStartByte;
		for (i in 0...numSamples) {
			var value = (data.get(b) << 8) | data.get(b + 1);
			rawData.set(b - 54, (value) & 0xff);
			rawData.set(b + 1 - 54, (value >> 8) & 0xff);
			b += 2;
		}
		this.samplingRate = Std.int(sampleRate);
		this.channels = 1;
		this.sampleFormat = SampleFormat.I16;
		this.samples = Std.int(rawData.length / getBytesPerSample());
	}

	override function decodeBuffer(out:haxe.io.Bytes, outPos:Int, sampleStart:Int, sampleCount:Int) {
		var bpp = getBytesPerSample();
		out.blit(outPos, rawData, sampleStart * bpp, sampleCount * bpp);
	}
}

class SoundRes extends hxd.res.Sound {
	public function new(data:Data) {
		super(null);
		this.data = data;
		this.entry = new Entry();
	}

	override function getData():Data {
		return data;
	}
}

class Entry extends FileEntry {
	public function new() {};

	override function get_path():String {
		return "";
	}
}
