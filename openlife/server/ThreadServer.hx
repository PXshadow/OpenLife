package openlife.server;

import openlife.macros.Macro;
import haxe.io.Eof;
import openlife.settings.ServerSettings;
#if (target.threaded)
import haxe.Timer;
import haxe.Exception;
import sys.net.Socket;
import sys.thread.Thread;
import sys.thread.Mutex;
import sys.net.Host;

class ThreadServer {
	public var socket:Socket;
	public var port:Int = 8005;
	public var maxCount:Int = -1;
	public var listenCount:Int = 10;

	public static inline var setTimeout:Int = 30;

	public var server:Server;

	public function new(server:Server, port:Int) {
		socket = new Socket();
		this.port = port;
		this.server = server;
	}

	public function create() {
		socket.bind(new Host("0.0.0.0"), port);
		trace('listening on port: $port');
		socket.listen(listenCount);
		while (true) {
			Thread.create(connection).sendMessage(socket.accept());
		}
	}

	private function connection() {
		var socket:Socket = cast Thread.readMessage(true);
		trace("start connection");
		socket.setBlocking(ServerSettings.UseBlockingSockets);
		socket.setFastSend(true);
		var connection = new Connection(socket, server);
		var message:String = "";
		var ka:Float = Timer.stamp();

		while (connection.running) {
			try {
				Sys.sleep(0.1);

				message = socket.input.readUntil("#".code);

				// trace(message);
				ka = Timer.stamp();
			} catch (e:Dynamic) {
				// error("---STACK---\n" + e.details());

				if (e != haxe.io.Error.Blocked) {
					if ('$e' == 'Eof') {
						trace('Client closed connection / EOF');
					} else
						trace('WARNING: EXEPTION: ' + e);

					Macro.exception(connection.close());

					break;
				} else {
					if (Timer.stamp() - ka > 20) {
						Macro.exception(connection.close());
					}
				}
			}

			if (message.length == 0) continue;

			if (ServerSettings.debug) {
				server.process(connection, message);
				message = "";
			} else {
				try {
					server.process(connection, message);
					message = "";
				} catch (e:Exception) {
					trace(e.details());
					error("---STACK---\n" + e.details());
					// connection.close();
					continue;
				}
			}
		}
	}

	private function error(message:String) {}
}
#end
