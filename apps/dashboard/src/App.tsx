import { Route, Routes } from 'react-router-dom';
import { ModeProvider } from './state/ModeContext';
import { ThemeProvider } from './state/ThemeContext';
import Layout from './components/Layout';
import CommandCenter from './pages/CommandCenter';
import PnlAnalytics from './pages/PnlAnalytics';
import PnlCalendar from './pages/PnlCalendar';
import Portfolio from './pages/Portfolio';
import Execution from './pages/Execution';

export default function App() {
  return (
    <ThemeProvider>
      <ModeProvider>
        <Routes>
          <Route element={<Layout />} path="/">
            <Route index element={<CommandCenter />} />
            <Route path="pnl" element={<PnlAnalytics />} />
            <Route path="calendar" element={<PnlCalendar />} />
            <Route path="portfolio" element={<Portfolio />} />
            <Route path="execution" element={<Execution />} />
          </Route>
        </Routes>
      </ModeProvider>
    </ThemeProvider>
  );
}
