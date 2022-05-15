package scripts;

import scripts.SetupData;

class SetupDataClient {
	public static function main() {
		new SetupDataClient();
	}

	public function new() {
		SetupGameData();
		SetupGameSourceData();
	}
}
