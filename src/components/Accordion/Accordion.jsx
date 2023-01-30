import React, { useState } from 'react';
import "./Accordion.css"
import arrow from "./accordion-arrow.svg"

function Accordion({ sections }) {
  // const [activeIndex, setActiveIndex] = useState(null);
  const [rotated, setRotated] = useState(Array(sections.length).fill(false));

  const handleClick = (index) => {
    // setActiveIndex(index === activeIndex ? null : index);
    setRotated(rotated.map((r, i) => i === index ? !r : r));
  };

  return (
    <div>
      {sections.map((section, index) => (
        <div className='accordion' key={index}>
          <button onClick={() => handleClick(index)}>
            <span>{section.title}</span>
            <span>
              <img src={arrow} alt="" className={rotated[index] ? "rotate" : "unrotate"} />
            </span>
          </button>
          {rotated[index] && <div className='content'>{section.content}</div>}
        </div>
      ))}
    </div>
  );
}

export default Accordion;
