import ReactDOM from 'react-dom/client';
import { App } from './App';
import { AppErrorBoundary } from './AppErrorBoundary';
import { AppButton } from './design/AppButton';
import {installAppCloseGuard} from './lib/appClose';
import {installAppZoom} from './lib/appZoom';
import '@radix-ui/themes/styles.css';
import '@xyflow/react/dist/style.css';
import './App.css';
import './theme.css';

const root = ReactDOM.createRoot(document.getElementById('root')!);
async function start() {
  try {
    await installAppCloseGuard();
    await installAppZoom();
    root.render(<AppErrorBoundary><App /></AppErrorBoundary>);
  } catch {
    root.render(<main className="app-error-fallback" role="alert"><div className="app-error-card">
      <h1>Code Atlas could not finish opening.</h1>
      <p>Your saved explanations are preserved. Try opening the workspace again.</p>
      <AppButton intent="primary" className="" onClick={()=>{void start();}}>Try again</AppButton>
    </div></main>);
  }
}
void start();
