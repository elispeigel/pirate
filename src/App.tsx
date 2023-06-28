import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

function App() {
  const [fileContents, setFileContents] = useState<Uint8Array>();
  const [filePath, setFilePath] = useState("");

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    console.log("🚀 ~ file: App.tsx:11 ~ handleFileChange ~ file:", file)
    if (file) {
      const fileUrl = URL.createObjectURL(file);
      console.log("🚀 ~ file: App.tsx:13 ~ handleFileChange ~ fileUrl:", fileUrl)
      setFilePath(fileUrl);
    }
  };

  const handleClick = async () => {
    try {
      const result: Uint8Array = await invoke("start");
      console.log("🚀 ~ file: App.tsx:17 ~ handleClick ~ result:", result)
      setFileContents(result);
    } catch (error) {
      console.error(error);
    }
  };

  return (
    <div className="container">
      <h1>Welcome to Tauri!</h1>

      <div>
        <input type="file" onChange={handleFileChange} />
        <button onClick={handleClick} disabled={!filePath}>
          Read File
        </button>
        <pre>{fileContents}</pre>
      </div>
    </div>
  );
}

export default App;
