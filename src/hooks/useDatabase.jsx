import Database from "@tauri-apps/plugin-sql";
import { useState } from "react";

function useDatabase() {
    /** @type {[Database, React.Dispatch<React.SetStateAction<Database>>]} */
    const [database, setDatabase] = useState(null);
    Database.load("sqlite:test.db").then(setDatabase);
    return database
}

export default useDatabase;
