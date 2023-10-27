package openlife.server;

import openlife.macros.Macro;
import openlife.data.object.ObjectData;
import haxe.Resource;
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
	var lineageText:String = 'Loading Lineages...<br>\n';
	var foodText:String = 'Loading Food Statistics...<br>\n';
	var accountsText:String = 'Loading account scores<br>\n';

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
			Macro.exception(acceptConnection(serverSocket));
		}
	}

	private function acceptConnection(serverSocket:Socket) {
		var socket = serverSocket.accept();
		socket.setBlocking(false);
		socket.setFastSend(true);
		handleRequest(socket);
	}

	public function createCurrentlyPlayingStatistics() {
		var countHuman = 0;
		var countAi = 0;
		var countStarving = 0;
		var livingPlayerText = '<table>\n<tr><td><b>Name</b></td><td><b>Age</b></td><td><b>Prestige</b></td><td><b>Power</b></td><td><b>Generation</b></td></tr>\n';

		GlobalPlayerInstance.AcquireMutex();
		for (player in GlobalPlayerInstance.AllPlayers) {
			if (player.isDeleted()) continue;
			if (player.food_store < 1) countStarving++;
			if (player.isHuman()) countHuman++;
			else countAi++;

			var lineage = player.lineage;
			livingPlayerText += '<tr>';
			livingPlayerText += '<td><font color="${getPersonFontColor(player)}">${lineage.getFullName()}</font></td>';
			livingPlayerText += '<td>${Math.floor(player.trueAge)}</td>';
			livingPlayerText += '<td>${Math.floor(player.prestige)}</td>';
			livingPlayerText += '<td>${Math.floor(player.power)}</td>';
			livingPlayerText += '<td>${lineage.generation}</td>';
			livingPlayerText += '</tr>\n';
		}
		// var count = Connection.CountHumans();
		GlobalPlayerInstance.ReleaseMutex();

		livingPlayerText += '</table></center>';

		var countText = '<center><p><b>Currently Playing: ${countHuman}\n';
		countText += '&Tab;AIs: ${countAi}\n';
		countText += '&Tab;Starving: ${countStarving}\n';
		countText += '&Tab;Season: ${TimeHelper.SeasonText}</b></p>\n';

		livingPlayerText = countText + livingPlayerText;

		this.countHuman = countHuman;
		this.countAi = countAi;
		this.countStarving = countStarving;
		this.livingPlayerText = livingPlayerText;
	}

	private function getPersonFontColor(player:GlobalPlayerInstance) {
		// person ==> Ginger = 6 / White = 4 / Brown = 3 /  Black = 1
		var color = player.getColor();

		if (color == 1) return "#8B80001"; // Yellow FFFF00
		if (color == 3) return "#008000"; // Green
		if (color == 4) return "#808080"; // Grey
		return "#FFFFFF"; // White
	}

	public function generateAccountStatistics() {
		var count = 0;
		var countHuman = 0;
		var newAccountsText = '';

		for (account in PlayerAccount.AllPlayerAccountsById) {
			count++;

			if (account.isAi) continue;
			if (account.isAi == false) countHuman++;
			if (account.totalScore < 5) continue;

			newAccountsText += '<tr>';
			newAccountsText += '<td>${account.id}</td>';
			newAccountsText += '<td>${account.totalScore}</td>';
			newAccountsText += '<td>${Math.floor(account.femaleScore)}</td>';
			newAccountsText += '<td>${Math.floor(account.maleScore)}</td>';
			newAccountsText += '<td>${Math.floor(account.coinsInherited)}</td>';
			newAccountsText += '</tr>\n';
		}
		newAccountsText += '</table></center';
		accountsText = '<br><br><center>Score: count: ${count} human: ${countHuman}\n\n<table>\n<tr><td><b>ID</b></td><td><b>Prestige</b></td><td><b>Female Prestige</b></td><td><b>Male Prestige</b></td><td><b>Coins</b></td></tr>\n';
		accountsText += newAccountsText;
	}

	public function generateLineageStatistics() {
		GlobalPlayerInstance.AcquireMutex();
		var done = Lineage.GenerateLineageStatistics();
		GlobalPlayerInstance.ReleaseMutex();

		if (done == false) return;

		var reasonKilled = Lineage.reasonKilled;
		var ages = Lineage.ages;
		var generations = Lineage.generations;

		var reasonKilledList = [for (a in reasonKilled.keys()) a];
		reasonKilledList.sort(function(a, b) {
			if (a < b) return -1;
			else if (a > b) return 1;
			else return 0;
		});

		lineageText = '<br><br>\n<center><table>\n<tr><td><b>Reason killed</b></td><td><b>Total</b></td><td><b>Last Day</b></td><td><b>Last Hour</b></td></tr>\n';

		for (reason in reasonKilledList) {
			var reasonText = reason;
			if (reasonText == 'null') reasonText = 'N/A';
			else if (reasonText == '') continue;
			else if (reasonText == 'reason_age') reasonText = 'OLD AGE';
			else if (reasonText == 'reason_hunger') reasonText = 'STARVATION';

			lineageText += '<tr>';
			lineageText += '<td>${reasonText}</td>';
			lineageText += '<td>${Lineage.reasonKilled[reason]}</td>';
			lineageText += '<td>${cast (Lineage.reasonKilledLastDay[reason], Int)}</td>';
			lineageText += '<td>${cast (Lineage.reasonKilledLastHour[reason], Int)}</td>';
			lineageText += '</tr>\n';
		}

		lineageText += '</table></center>\n';
		lineageText += '<br><br>\n<center><table>\n<tr><td><b>Age</b></td><td><b>Total</b></td><td><b>Last Day</b></td><td><b>Last Hour</b></td></tr>\n';

		var ageList = [for (a in ages.keys()) a];
		ageList.sort(function(a, b) {
			if (a < b) return -1;
			else if (a > b) return 1;
			else return 0;
		});

		for (age in ageList) {
			var ageText;
			if (age < 0) ageText = 'N/A';
			else ageText = '${age}';

			lineageText += '<tr>';
			lineageText += '<td>Age: ${ageText}</td>';
			lineageText += '<td>${Lineage.ages[age]}</td>';
			lineageText += '<td>${cast (Lineage.agesLastDay[age], Int)}</td>';
			lineageText += '<td>${cast (Lineage.agesLastHour[age], Int)}</td>';
			lineageText += '</tr>\n';
		}

		lineageText += '</table></center>\n';
		/*lineageText += '<br><br>\n<center><table>\n<tr><td><b>Generation</b></td><td><b>Count<b></td></tr>\n';

			var generationsList = [for (g in generations.keys()) g];

			generationsList.sort(function(a, b) {
				if (a < b) return -1; else if (a > b) return 1; else
					return 0;
			});

			for (generation in generationsList) {
				lineageText += '<tr>';
				lineageText += '<td>Generation: ${generation}</td>';
				lineageText += '<td>${generations[generation]}</td>';
				lineageText += '</tr>\n';
			}

			lineageText += '</table></center>\n';
		 */
	}

	public function generateFoodStatistics() {
		// GlobalPlayerInstance.AcquireMutex();
		// var done = Lineage.GenerateLineageStatistics();
		// GlobalPlayerInstance.ReleaseMutex();

		var eatenFoodPercentage = WorldMap.world.eatenFoodPercentage;
		// eatenFoodPercentage[31] = 1;

		// TODO sort for percentage
		var eatenFoodPercenList = [for (a in eatenFoodPercentage.keys()) a];
		eatenFoodPercenList.sort(function(a, b) {
			if (a < b) return -1;
			else if (a > b) return 1;
			else return 0;
		});

		// <td><b>Last Hour</b></td>
		foodText = '<br><br>\n<center><table>\n<tr><td><b>Food</b></td><td><b>Eaten</b></td><td><b>Related</b></td></tr>\n';

		for (foodId in eatenFoodPercenList) {
			var objData = ObjectData.getObjectData(foodId);
			var foodName = objData.name;
			var foodTotalPercent:Int = Math.round(WorldMap.world.getEatenFoodPercentage(foodId));

			foodText += '<tr>';
			foodText += '<td>${foodName}</td>';
			foodText += '<td>${eatenFoodPercentage[foodId]}%</td>';
			foodText += '<td>${foodTotalPercent}%</td>';
			// foodText += '<td>${cast (Lineage.reasonKilledLastHour[reason], Int)}</td>';
			foodText += '</tr>\n';
		}

		foodText += '</table></center>\n';
	}

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
		Macro.exception(createCurrentlyPlayingStatistics());
		Macro.exception(generateLineageStatistics());
		Macro.exception(generateFoodStatistics());
		Macro.exception(generateAccountStatistics());

		// var text = '<!DOCTYPE html>\n<html>\n<head>\n<title>Open Life Reborn</title>\n</head>\n<body>\n<h1>Welcome to Open Life Reborn!</h1><p>Currently Playing: ${count}</p>\n</body>\n</html>';
		var text = welcomeText;
		// text = text.replace('</ul>', '</ul>\n<p>Currently Playing: ${count}</p>');
		text = text.replace('</body>', '${accountsText}\n${livingPlayerText}\n${foodText}\n${lineageText}\n</body>');

		var message = 'HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Encoding: UTF-8\r\nContent-Length: ${text.length}\r\n\r\n${text}';
		// var message = "HTTP/1.1 200 OK\nContent-Type: text/html; charset=UTF-8\nContent-Encoding: UTF-8\nContent-Length: ${text.length}\nDate: Wed, 28 Jun 2023 22:36:00 GMT+02:00\n\n<!DOCTYPE html>\n<html>\n<head>\n    <title>Example</title>\n</head>\n<body>\n    <h1>Hello World!</h1>\n</body>\n</html>";

		return message;
	}
}
