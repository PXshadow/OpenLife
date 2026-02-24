package openlife.server;

import openlife.macros.Macro;
import openlife.data.object.ObjectData;
import openlife.settings.ServerSettings;
import sys.net.Host;
import sys.net.Socket;
import sys.thread.Thread;

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
		var path = dir + ServerSettings.WebServerMainHtml;

		if (!sys.FileSystem.exists(path)) {
			trace('Could not find welcome text!');
			return;
		}

		welcomeText = sys.io.File.getContent(path);

		Thread.create(function() {
			this.run();
		});
	}

	public function run() {
		var port = 80;
		var serverSocket = new Socket();
		serverSocket.bind(new Host("0.0.0.0"), port);
		trace('listening on port: $port');
		serverSocket.listen(listenCount);

		while (true) {
			Macro.exception(acceptConnection(serverSocket));
		}
	}

	private function acceptConnection(serverSocket:Socket) {
		var socket = serverSocket.accept();
		// Handle each connection on its own thread so slow clients
		// don't block the accept loop.
		Thread.create(function() {
			Macro.exception(handleConnection(socket));
		});
	}

	// -------------------------------------------------------------------------
	// HTTP request parsing and routing
	// -------------------------------------------------------------------------

	private function handleConnection(socket:Socket) {
		try {
			socket.setFastSend(true);

			// Read until the blank line that ends the HTTP request headers.
			var buf = new StringBuf();
			var input = socket.input;
			while (true) {
				buf.addChar(input.readByte());
				var s = buf.toString();
				if (s.endsWith('\r\n\r\n') || s.endsWith('\n\n')) break;
			}

			// First line is: METHOD /path HTTP/1.x
			var firstLine = buf.toString().split('\n')[0].trim();
			var parts = firstLine.split(' ');
			var method = parts.length > 0 ? parts[0].toUpperCase() : 'GET';
			var path = parts.length > 1 ? parts[1] : '/';

			// Strip query string
			var qIdx = path.indexOf('?');
			if (qIdx >= 0) path = path.substr(0, qIdx);

			trace('$method $path');

			var response = routeRequest(method, path);
			socket.output.writeString(response);
			socket.output.flush();
		} catch (e:Dynamic) {
			trace('WebServer connection error: $e');
			try {
				socket.output.writeString(buildResponse(500, "Internal Server Error", "text/plain", "Internal Server Error"));
				socket.output.flush();
			} catch (_:Dynamic) {}
		}

		try {
			socket.close();
		} catch (_:Dynamic) {}
	}

	private function routeRequest(method:String, path:String):String {
		if (method != 'GET') {
			return buildResponse(405, "Method Not Allowed", "text/plain", "Method Not Allowed");
		}

		return switch path {
			case '/' | '/index.html':
				Macro.exception(createCurrentlyPlayingStatistics());
				Macro.exception(generateLineageStatistics());
				Macro.exception(generateFoodStatistics());
				Macro.exception(generateAccountStatistics());
				var html = buildPageHtml();
				fullLandingPageText = html;
				buildResponse(200, "OK", "text/html; charset=UTF-8", html);

			case '/stats/players':
				Macro.exception(createCurrentlyPlayingStatistics());
				buildResponse(200, "OK", "text/html; charset=UTF-8", livingPlayerText != null ? livingPlayerText : "Loading...");

			case '/stats/lineage':
				Macro.exception(generateLineageStatistics());
				buildResponse(200, "OK", "text/html; charset=UTF-8", lineageText);

			case '/stats/food':
				Macro.exception(generateFoodStatistics());
				buildResponse(200, "OK", "text/html; charset=UTF-8", foodText);

			case '/stats/accounts':
				Macro.exception(generateAccountStatistics());
				buildResponse(200, "OK", "text/html; charset=UTF-8", accountsText);

			default:
				buildResponse(404, "Not Found", "text/plain", "Not Found");
		}
	}

	/**
	 * Builds a complete HTTP/1.1 response string with correct headers.
	 * Content-Length is based on the UTF-8 byte length of the body.
	 */
	private function buildResponse(status:Int, reason:String, contentType:String, body:String):String {
		var byteLen = haxe.io.Bytes.ofString(body).length;
		return 'HTTP/1.1 $status $reason\r\n'
			+ 'Content-Type: $contentType\r\n'
			+ 'Content-Length: $byteLen\r\n'
			+ 'Content-Security-Policy: default-src \'self\' \'unsafe-inline\' \'unsafe-eval\'\r\n'
			+ 'X-Content-Type-Options: nosniff\r\n'
			+ 'Connection: close\r\n'
			+ '\r\n'
			+ body;
	}

	private function buildPageHtml():String {
		if (welcomeText == null) return "<html><body>Server not ready.</body></html>";

		var text = welcomeText;
		text = text.replace("${livingPlayerText} ${accountsText} ${foodText} ${lineageText}",
			'${livingPlayerText}\n${accountsText}\n${foodText}\n${lineageText}\n</body>');
		return text;
	}

	// -------------------------------------------------------------------------
	// Statistics helpers (logic unchanged from original)
	// -------------------------------------------------------------------------

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
		var accountList = new Array<PlayerAccount>();

		for (account in PlayerAccount.AllPlayerAccountsById) {
			count++;
			if (account.isAi) continue;
			if (account.isAi == false) countHuman++;
			if (account.totalScore < 5) continue;
			accountList.push(account);
		}

		accountList.sort(function(a, b) {
			if (a.totalScore < b.totalScore) return 1;
			else if (a.totalScore > b.totalScore) return -1;
			else return 0;
		});

		for (account in accountList) {
			count++;

			if (account.isAi) continue;
			if (account.isAi == false) countHuman++;
			if (account.totalScore < 5) continue;

			newAccountsText += '<tr>';
			newAccountsText += '<td>${account.scoreName}</td>';
			newAccountsText += '<td>${account.totalScore}</td>';
			newAccountsText += '<td>${Math.floor(account.femaleScore)}</td>';
			newAccountsText += '<td>${Math.floor(account.maleScore)}</td>';
			newAccountsText += '<td>${Math.floor(account.coinsInherited)}</td>';
			newAccountsText += '</tr>\n';
		}
		newAccountsText += '</table></center>';
		accountsText = '<br><br><center>Score: count: ${count} human: ${countHuman}\n\n<table>\n<tr><td><b>ID</b></td><td><b>Prestige</b></td><td><b>Female Prestige</b></td><td><b>Male Prestige</b></td><td><b>Coins</b></td></tr>\n';
		accountsText += newAccountsText;
	}

	public function generateAccountName(id:Int):String {
		var length = NamingHelper.MaleNamesArray.length;
		var index = WorldMap.RandomIntFromSeed(length, id * 100 + id + 42);
		var name = NamingHelper.MaleNamesArray[index];

		index = (id * 100 + id + 9973) % NamingHelper.FemaleNamesArray.length;
		name += ' ' + NamingHelper.FemaleNamesArray[index];
		return name;
	}

	public function generateLineageStatistics() {
		GlobalPlayerInstance.AcquireMutex();
		var done = Lineage.GenerateLineageStatistics();
		GlobalPlayerInstance.ReleaseMutex();

		if (done == false) return;

		var reasonKilled = Lineage.reasonKilled;
		var ages = Lineage.ages;

		var reasonKilledList = [for (a in reasonKilled.keys()) a];
		reasonKilledList.sort(function(a, b) return (a < b) ? -1 : (a > b) ? 1 : 0);

		lineageText = '<br><br>\n<center><table>\n<tr><td><b>Reason killed</b></td><td><b>Total</b></td><td><b>Last Day</b></td><td><b>Last Hour</b></td></tr>\n';

		for (reason in reasonKilledList) {
			var reasonText = reason;
			if (reasonText == 'null') reasonText = 'N/A';
			else if (reasonText == '') continue;
			else if (reasonText == 'reason_age') reasonText = 'OLD AGE';
			else if (reasonText == 'reason_hunger') reasonText = 'STARVATION';
			else if (reasonText == 'reason_hunger_kid') reasonText = 'STARVATION KID';

			lineageText += '<tr>';
			lineageText += '<td>${reasonText}</td>';
			lineageText += '<td>${Lineage.reasonKilled[reason]}</td>';
			lineageText += '<td>${cast (Lineage.reasonKilledLastDay[reason], Int)}</td>';
			lineageText += '<td>${cast (Lineage.reasonKilledLastHour[reason], Int)}</td>';
			lineageText += '</tr>\n';
		}

		var foodFactorPercent = Math.ceil(WorldMap.world.getStarvingFoodFactor() * 100) - 100;
		lineageText += '</table>Extra food because of Starving: ${foodFactorPercent}%</center>\n';
		lineageText += '<br><br>\n<center><table>\n<tr><td><b>Age</b></td><td><b>Total</b></td><td><b>Last Day</b></td><td><b>Last Hour</b></td></tr>\n';

		var ageList = [for (a in ages.keys()) a];
		ageList.sort(function(a, b) return (a < b) ? -1 : (a > b) ? 1 : 0);

		for (age in ageList) {
			var ageText = (age < 0) ? 'N/A' : '${age}';
			lineageText += '<tr>';
			lineageText += '<td>${ageText}</td>';
			lineageText += '<td>${Lineage.ages[age]}</td>';
			lineageText += '<td>${cast (Lineage.agesLastDay[age], Int)}</td>';
			lineageText += '<td>${cast (Lineage.agesLastHour[age], Int)}</td>';
			lineageText += '</tr>\n';
		}

		lineageText += '</table></center>\n';
	}

	public function generateFoodStatistics() {
		var eatenFoodPercentage = WorldMap.world.eatenFoodPercentage;

		var eatenFoodPercenList = [for (a in eatenFoodPercentage.keys()) a];
		eatenFoodPercenList.sort(function(a, b) return (a < b) ? -1 : (a > b) ? 1 : 0);

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
}
