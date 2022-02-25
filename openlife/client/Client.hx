package openlife.client;

import haxe.io.BytesBuffer;
import openlife.settings.Settings.ConfigData;
import haxe.io.Bytes;
import openlife.client.ClientTag;
import sys.io.File;
#if sys
import sys.net.Socket;
#else
import js.node.net.Socket;
#end
import sys.net.Host;
import haxe.io.Error;
import haxe.crypto.Hmac;
import haxe.Timer;

/**
 * Socket Client
 */
@:expose
class Client {
	var socket:Socket;
	// interact to be able to login to game
	var data:String = "";
	var aliveStamp:Float = 0;
	var connected:Bool = false;

	public var message:(tag:ClientTag, input:Array<String>) -> Void;
	public var onClose:Void->Void;
	public var onReject:Void->Void;
	public var onAccept:Void->Void;
	// ping
	public var ping:Int = 0;

	var pingInt:Int = 0;

	public var config:ConfigData;

	var challenge:String;

	public var version:String;
	public var reconnect:Bool = false;
	// functions
	public var accept:Void->Void;
	public var reject:Void->Void;
	public var relayIn:Socket;
	public var relayServer:#if sys Socket #else js.node.net.Server #end;

	var wasCompressed:Bool = false;

	public function new() {
		aliveStamp = Timer.stamp();
	}

	#if sys
	public function update() {
		@:privateAccess haxe.MainLoop.tick(); // for timers
		if (Timer.stamp() - aliveStamp >= 15) alive();
		if (!connected) return;
		data = "";
		if (relayIn != null) {
			// relay system embeded into client update
			try {
				@:privateAccess var input = relayIn.input.readUntil("#".code);
				send(input);
			} catch (e) {
				if (e.message != "Blocked") close();
			}
		}
		try {
			if (compressSize > 0) {
				var temp = socket.input.read(compressSize - compressIndex);
				if (compressInput(temp)) return;
			} else {
				data = socket.input.readUntil("#".code);
			}
		} catch (e:haxe.Exception) {
			if (e.message != "Blocked") {
				trace("e " + e.message);
				if (e.details().indexOf('Eof') > -1) {
					connected = false;
					data = "";
					close();
				} else {
					trace('e: ${e.details()}');
					close();
				}
			}
			return;
		}
		process(wasCompressed);
		wasCompressed = false;
		update();
	}
	#else
	var inputData:Array<js.node.Buffer> = [];

	function inputDataLength():Int {
		var int = 0;
		for (input in inputData) {
			int += input.byteLength;
		}
		return int;
	}

	function inputDataGetBytes():Bytes {
		var bytes = new BytesBuffer();
		for (input in inputData) {
			bytes.add(input.hxToBytes());
		}
		return bytes.getBytes();
	}

	function update(buffer:js.node.Buffer, addition:Bool = false) {
		trace(buffer.toString());
		if (!addition) relayIn.write(buffer);
		var index = 0;
		if (compressSize > 0) {
			var tmp = buffer.slice(0, compressSize - compressIndex);
			inputData.push(tmp.slice(tmp.length));
			if (compressInput(tmp.hxToBytes())) return;
			index = tmp.length;
		} else {
			index = buffer.indexOf("#");
			if (index == -1) {
				inputData.push(buffer);
				return;
			}
			inputData.push(buffer.slice(0, index));
			var bytes = inputDataGetBytes();
			data = bytes.toString();
			index += 1;
		}
		process(wasCompressed);
		wasCompressed = false;
		inputData = [];
		buffer = buffer.slice(index);
		if (buffer.length == 0) return;
		update(buffer, true);
	}
	#end

	function compressInput(temp:Bytes):Bool {
		dataCompressed.blit(compressIndex, temp, 0, temp.length);
		compressIndex += temp.length;
		if (compressIndex >= compressSize) {
			compressProcess();
			compressIndex = 0;
			compressSize = 0;
			data = haxe.zip.Uncompress.run(dataCompressed).toString();
			wasCompressed = true;
			if (tag == MAP_CHUNK) {
				data = '$MAP_CHUNK\n$data';
			}
		} else {
			return true;
		}
		return false;
	}

	var listen:Int;

	public function relay(listen:Int) {
		this.listen = listen;
		Sys.println('waiting for connection on port $listen');
		#if nodejs
		relayServer = js.node.Net.createServer(function(c) {
			relayIn = c;
			relayIn.setNoDelay(true);
			relayIn.on('data', function(buffer) {
				socket.write(buffer);
			});
			relayIn.on(js.node.net.Socket.SocketEvent.End, function() {
				trace("relayIn failed");
				close();
			});
		});
		relayServer.listen(listen);
		Sys.println("node sync wait");
		sys.NodeSync.wait(function() {
			return relayIn != null;
		});
		#else
		relayServer = new Socket();
		relayServer.bind(new Host("localhost"), listen);
		relayServer.listen(1);

		relayIn = relayServer.accept();
		// here we are connected
		relayIn.setFastSend(true);
		relayIn.setBlocking(false);
		#end
	}

	var tag:ClientTag;

	private function process(wasCompressed:Bool) {
		// relay
		#if !nodejs
		if (!wasCompressed && relayIn != null) {
			relaySend(data);
		}
		#end
		// normal client
		var array = data.split("\n");
		if (array.length == 0) return;
		tag = array[0];
		message(tag, array.slice(1, array.length > 2 ? array.length - 1 : array.length));
	}

	private function compressProcess() {
		#if !nodejs
		if (relayIn != null) {
			relayIn.output.write(dataCompressed);
		}
		#end
	}

	public function alive() {
		send("KA 0 0");
		send("PING 0 0 " + pingInt++);
		aliveStamp = Timer.stamp();
	}

	public function login(tag:ClientTag, input:Array<String>) {
		// login process
		switch (tag) {
			case SERVER_INFO:
				// current
				// trace("amount " + input[0]);
				// challenge
				challenge = input[1];
				// version
				version = input[2];
				// trace("version " + version);
				request();
			case ACCEPTED:
				if (accept != null) accept();
				if (onAccept != null) onAccept();
			case REJECTED:
				trace("REJECTED LOGIN");
				if (reject != null) reject();
				if (onReject != null) onReject();
			default:
				trace('$tag not registered');
			case null:
				trace('tag not found in data:\n$data');
		}
	}

	private function request() {
		var key = StringTools.replace(config.key, "-", "");
		var email = config.email + (config.seed == "" ? "" : "|" + config.seed);
		var password = new Hmac(SHA1).make(Bytes.ofString("262f43f043031282c645d0eb352df723a3ddc88f"), Bytes.ofString(challenge)).toHex();
		var accountKey = new Hmac(SHA1).make(Bytes.ofString(key), Bytes.ofString(challenge)).toHex();
		var clientTag = " client_openlife";
		if (config.legacy) clientTag = "";
		var requestString = (reconnect ? "R" : "") + 'LOGIN$clientTag $email $password $accountKey ${(config.tutorial ? 1 : 0)}';
		send(requestString);
	}

	public function send(data:String) {
		if (!connected) return;
		#if nodejs
		socket.write('$data#');
		#else
		try {
			socket.output.writeString('$data#');
		} catch (e:Dynamic) {
			trace("client send error: " + e);
			close();
			return;
		}
		#end
	}

	public function relaySend(data:String) {
		try {
			#if !nodejs
			relayIn.output.writeString('$data#');
			#else
			relayIn.write('$data#');
			#end
		} catch (e:Dynamic) {
			trace("client send error: " + e);
			close();
			return;
		}
	}

	var compressIndex:Int = 0;
	var dataCompressed:Bytes;
	var compressSize:Int = 0;
	var rawSize:Int = 0;

	public function compress(rawSize:Int, compressSize:Int) {
		this.rawSize = rawSize;
		this.compressSize = compressSize;
		dataCompressed = Bytes.alloc(compressSize);
		compressIndex = 0;
	}

	public function connect(reconnect:Bool = false) {
		if (config == null) {
			throw "config is null";
			return;
		}
		this.reconnect = reconnect;
		if (config.port == null) config.port = 8005;
		if (config.tutorial == null) config.tutorial = false;
		if (config.legacy == null) config.legacy = false;
		if (config.seed == null) config.seed = "";
		if (config.twin == null) config.twin = "";
		if (config.email == null) config.email = "test@email.email";
		if (config.key == null) config.key = "8888-8888-8888-8888";
		trace("attempt connect " + config.ip + ":" + config.port);
		connected = false;
		var host:Host;
		try {
			host = new Host(config.ip);
		} catch (e:Dynamic) {
			trace("host error: " + e);
			return;
		}
		#if sys
		socket = new Socket();
		try {
			socket.connect(host, config.port);
		} catch (e:Dynamic) {
			trace("socket connect error: " + e);
			close();
			return;
		}
		socket.setBlocking(false);
		#else
		socket = new Socket();
		socket.connect(config.port, host.host, function() {
			socket.setNoDelay(true);
			socket.on('data', update);
		});
		sys.NodeSync.wait(function() {
			return socket != null;
		});
		#end
		connected = true;
		trace("connected");
	}

	public function close() {
		#if sys
		try {
			socket.close();
			if (relayIn != null) {
				relayServer.close();
				relayIn.close();
			}
		} catch (e:Dynamic) {
			trace("failure to close socket " + e);
		}
		#else
		socket.destroy();
		if (relayIn != null) {
			relayServer.close();
			relayIn.destroy();
		}
		#end
		trace("socket disconnected");
		connected = false;
		if (onClose != null) onClose();
	}
}
