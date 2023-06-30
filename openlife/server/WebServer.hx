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

	// Statistics
	var countHuman = 0;
	var countAi = 0;
	var countStarving = 0;
	var livingPlayerText:String = null;

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

	public function createCurrentlyPlayingStatistics() {
		var countHuman = 0;
		var countAi = 0;
		var countStarving = 0;
		var livingPlayerText = '<center><table>\n<tr><td>Name</td><td>Prestige</td><td>Age</td><td>Power</td></tr>\n';

		/*
			<table>
			  <tr>
			<td>Row 1, Column 1</td>
			<td>Row 1, Column 2</td>
			  </tr>
			  <tr>
			<td>Row 2, Column 1</td>
			<td>Row 2, Column 2</td>
			  </tr>
			</table>
		 */

		GlobalPlayerInstance.AcquireMutex();
		for (player in GlobalPlayerInstance.AllPlayers) {
			if (player.isDeleted()) continue;
			if (player.food_store < 1) countStarving++;
			if (player.isHuman()) countHuman++; else
				countAi++;

			var lineage = player.lineage;
			livingPlayerText += '<tr>';
			livingPlayerText += '<td>${lineage.getFullName()}</td>';
			livingPlayerText += '<td>${Math.floor(player.prestige)}</td>';
			// livingPlayerText += '<td>${lineage.generation}</td>';
			livingPlayerText += '<td>${Math.floor(player.trueAge)}</td>';
			livingPlayerText += '<td>${Math.floor(player.power)}</td>';
			livingPlayerText += '</tr>\n';
		}
		// var count = Connection.CountHumans();
		GlobalPlayerInstance.ReleaseMutex();

		livingPlayerText += '</table></center>';

		var countText = '<p>Currently Playing: ${countHuman}\n';
		countText += '&Tab;AIs: ${countAi}\n';
		countText += '&Tab;Starving: ${countStarving}</p>\n';

		livingPlayerText = countText + livingPlayerText;

		this.countHuman = countHuman;
		this.countAi = countAi;
		this.countStarving = countStarving;
		this.livingPlayerText = livingPlayerText;
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

		// GlobalPlayerInstance.AcquireMutex();
		// var count = Connection.CountHumans();
		// GlobalPlayerInstance.ReleaseMutex();

		// TODO do only every 10 sec
		createCurrentlyPlayingStatistics();

		// var text = '<!DOCTYPE html>\n<html>\n<head>\n<title>Open Life Reborn</title>\n</head>\n<body>\n<h1>Welcome to Open Life Reborn!</h1><p>Currently Playing: ${count}</p>\n</body>\n</html>';
		var text = welcomeText;
		// text = text.replace('</ul>', '</ul>\n<p>Currently Playing: ${count}</p>');
		text = text.replace('</body>', '${livingPlayerText}</body>');

		var message = 'HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Encoding: UTF-8\r\nContent-Length: ${text.length}\r\n\r\n${text}';
		// var message = "HTTP/1.1 200 OK\nContent-Type: text/html; charset=UTF-8\nContent-Encoding: UTF-8\nContent-Length: ${text.length}\nDate: Wed, 28 Jun 2023 22:36:00 GMT+02:00\n\n<!DOCTYPE html>\n<html>\n<head>\n    <title>Example</title>\n</head>\n<body>\n    <h1>Hello World!</h1>\n</body>\n</html>";

		return message;
	}
}
