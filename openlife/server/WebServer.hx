package openlife.server;

import sys.net.Host;
import sys.thread.Thread;
import sys.net.Socket;

class WebServer {
	public var listenCount:Int = 10;
	public var startText:String = null;

	public static function Start() {
		var webServer = new WebServer();
		webServer.start();
		return webServer;
	}

	private function new() {}

	private function start() {
		trace('Starting WebServer...');
		// run run run Thread run run run
		// var thread = new ThreadServer(this, 8005);
		Thread.create(function() {
			this.run();
		});
	}

	public function run() {
		var port = 80;
		var serverSocket = new Socket();

		// serverSocket.bind(new Host("127.0.0.1"), port);
		serverSocket.bind(new Host("0.0.0.0"), port);
		trace('listening on port: $port');
		serverSocket.listen(listenCount);

		while (true) {
			var socket = serverSocket.accept();
			socket.setBlocking(false);
			socket.setFastSend(true);
			handleRequest(socket);
		}
	}

	/*static function main() {
		var server = new ServerSocket();
		server.bind("127.0.0.1", 8080);
		server.listen(handleRequest);
	}*/
	private function handleRequest(socket:Socket) {
		trace('received request!');

		if (startText == null) startText = createStartText();

		socket.output.writeString(startText);
		socket.close();
		startText = createStartText(); // TODO count once in a while
	}

	private static function createStartText() {
		var text = 'Welcome to Open Life Reborn!\n';

		GlobalPlayerInstance.AcquireMutex();
		var count = Connection.CountHumans();
		GlobalPlayerInstance.ReleaseMutex();

		text += 'Currently Playing: ${count}';
		var message = 'HTTP/1.1 200 OK\r\nContent-Length: ${text.length}\r\n\r\n$text';
		return message;
	}
}
