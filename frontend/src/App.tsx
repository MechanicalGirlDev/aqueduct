import { NodeEditor } from './components/NodeEditor';
import { Palette } from './components/Palette';
import { PropertyPanel } from './components/PropertyPanel';
import { RuntimeControls } from './components/RuntimeControls';
import { ThemeProvider } from './components/theme-provider';
import { useGraphSync } from './hooks/useGraphSync';
import { useTransport } from './hooks/useTransport';

function AppContent() {
  const { transport, connected } = useTransport();
  useGraphSync();

  return (
    <div className="flex h-screen w-screen flex-col bg-background">
      <RuntimeControls transport={transport} connected={connected} />

      <main className="flex min-h-0 flex-1 max-md:flex-col">
        <div className="w-[250px] min-h-0 border-r border-border max-md:w-full max-md:min-h-[160px] max-md:max-h-[220px] max-md:border-r-0 max-md:border-b">
          <Palette />
        </div>

        <div className="flex-1 min-h-0 max-md:min-h-[320px]">
          <NodeEditor />
        </div>

        <div className="w-[300px] min-h-0 border-l border-border max-md:w-full max-md:min-h-[160px] max-md:max-h-[220px] max-md:border-l-0 max-md:border-t">
          <PropertyPanel />
        </div>
      </main>
    </div>
  );
}

function App() {
  return (
    <ThemeProvider defaultTheme="light">
      <AppContent />
    </ThemeProvider>
  );
}

export default App;
