package openlife.server;

import openlife.settings.ServerSettings;
import sys.net.Host;
import sys.thread.Thread;
import sys.net.Socket;
import sys.io.File;
import sys.io.FileInput;

using StringTools;

class WebServer {
	public var listenCount:Int = 10;
	public var welcomeText:String = null;
	public var fullLandingPageText:String = null;

	public static function Start() {
		var webServer = new WebServer();
		webServer.start();
		return webServer;
	}

	private function new() {}

	private function start() {
		trace('Starting WebServer...');

		var dir = './${ServerSettings.WebServerDirectory}/';
		var path = dir + "OpenLifeReborn.html";
		var saveExists = sys.FileSystem.exists(path);

		if (saveExists == false) {
			trace('Could not find welcome text!');
			return;
		}

		var content:String = sys.io.File.getContent(path);
		// var reader = File.read(path, false);
		welcomeText = content;
		// trace(content);

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

		fullLandingPageText = createStartText();

		socket.output.writeString(fullLandingPageText); // TODO cache
		Sys.sleep(0.1);
		socket.close();
	}

	private function createStartText() {
		// var text = 'Welcome to Open Life Reborn!\n';

		GlobalPlayerInstance.AcquireMutex();
		var count = Connection.CountHumans();
		GlobalPlayerInstance.ReleaseMutex();

		// var text = '<!DOCTYPE html>\n<html>\n<head>\n<title>Open Life Reborn</title>\n</head>\n<body>\n<h1>Welcome to Open Life Reborn!</h1><p>Currently Playing: ${count}</p>\n</body>\n</html>';
		var text = welcomeText;
		text = text.replace('</ul>', '</ul>\n<p>Currently Playing: ${count}</p>');

		var message = 'HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Encoding: UTF-8\r\nContent-Length: ${text.length}\r\n\r\n${text}';
		// var message = "HTTP/1.1 200 OK\nContent-Type: text/html; charset=UTF-8\nContent-Encoding: UTF-8\nContent-Length: ${text.length}\nDate: Wed, 28 Jun 2023 22:36:00 GMT+02:00\n\n<!DOCTYPE html>\n<html>\n<head>\n    <title>Example</title>\n</head>\n<body>\n    <h1>Hello World!</h1>\n</body>\n</html>";

		return message;
	}
}
