import './App.css';
import Header from "./components/Header/Header"
import Footer from "./components/Footer/Footer"
import WhySection from "./components/WhySection/whySection.jsx"
import { BrowserRouter, Routes, Route } from "react-router-dom";
import './index.css';
import Home from './pages/home'
import Installation from './pages/installation'
import HeroSection from "./components/HeroSection/heroSection"

function App() {
  return (
    <BrowserRouter>
      <div className="App">
        <Header />
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/installation" element={<Installation />} />
          {/* <Route path="*" element={<NoPage />} /> */}
        </Routes>
        <WhySection />
        <Footer />
      </div>
    </BrowserRouter>
  );
}

export default App;
