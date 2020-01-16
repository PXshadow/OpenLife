package scripts;

import sys.db.Manager;

class DatabasePull
{
    public static function main()
    {
        Manager.cnx = sys.db.Mysql.connect({
            host:"localhost",
            port: null,
            user: "testUser",
            pass: "testPassword",
            database: "map",
            socket: null,
        });
    }
}