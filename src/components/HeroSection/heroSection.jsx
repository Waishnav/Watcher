import { Link } from "react-router-dom";
import './heroSection.css';
import screen_glow from "./screen-glow.svg"
import arrow from "./arrow.svg"

function heroSection() {
    return (
        <section className="hero-section">
          <div className="glowy-screen">
            <img className="" draggable="false" src={screen_glow} alt="" />
          </div>
          <div className="text">
              <h3>Minimal Open Source</h3>
              <h1>Screen-Time Tracker</h1>
              <p>Get a perspecitve from Watcher about your screen-time in a breif.
                    So it will help you to improve your productive-time. and makes your understand about where you spend your time in a day.
              </p>
              <Link className="primary-btn" to="/installation">
                  <span>Get It Now</span>
                  <span>
                    <img className="next-arrow" src={arrow} alt="" />
                  </span>
              </Link>          
          </div>
        </section>
    )
}

export default heroSection;
