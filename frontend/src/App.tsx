import { NodeEditor } from './components/NodeEditor';
import { Palette } from './components/Palette';
import { PropertyPanel } from './components/PropertyPanel';
import { RuntimeControls } from './components/RuntimeControls';
import { useGraphSync } from './hooks/useGraphSync';
import { useTransport } from './hooks/useTransport';

function App() {
  const { transport, connected } = useTransport();
  useGraphSync();

  return (
    <div className="app-shell">
      <RuntimeControls transport={transport} connected={connected} />

      <main className="app-main">
        <div className="app-palette">
          <Palette />
        </div>

        <div className="app-editor">
          <NodeEditor />
        </div>

        <div className="app-property">
          <PropertyPanel />
        </div>
      </main>
    </div>
  );
}

export default App;
