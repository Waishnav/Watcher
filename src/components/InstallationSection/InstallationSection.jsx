import { Link } from "react-router-dom";
import './InstallationSection.css';
import screen_glow from "../HeroSection/screen-glow.svg"
import arrow from "../HeroSection/arrow.svg"
import CodeBlock from "../CodeBlock/CodeBlock";

function InstallationSection() {
    return (
        <section className="installation-section">
          <div className="text">
            <h1>INSTALLATION STEPS</h1>
            <li>Copy the Following Command and paste in terminal</li>
            <CodeBlock code={`$ bash <(curl -s https://raw.githubusercontent.com/Waishnav/Watcher/main/install)`} />
            <li>Now its time to install the dependancies
              <ul>
                1. xdotool
              </ul>
              <ul>
                2. xprintidle
              </ul>
            </li>
            <CodeBlock code={`$ sudo [package-manager] install xprintidle xdotool`} />
            <p></p>
            <Link className="primary-btn" to="/">
              <span><img className="back-arrow" src={arrow} alt="" /></span>
              <span>Back</span>
            </Link>          
          </div>
        </section>
    )
}

export default InstallationSection;
