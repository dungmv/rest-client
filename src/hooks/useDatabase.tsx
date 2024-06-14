import Database from "@tauri-apps/plugin-sql";
import { useEffect, useState } from "react";

function useDatabase() {
    /** @type {[Database, React.Dispatch<React.SetStateAction<Database>>]} */
    const [database, setDatabase] = useState(null);
    useEffect(() => {
        Database.load("sqlite:restclient.db").then(setDatabase).catch(console.error);
    }, []);
    return database
}

export default useDatabase;
