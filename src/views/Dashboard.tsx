import InputBox from "../components/InputBox";
import Heatmap from "../components/Heatmap";
import Calendar from "../components/Calendar";

export default function Dashboard() {
  return (
    <div className="dash">
      <InputBox />
      <div className="dash-right">
        <Heatmap />
        <Calendar />
      </div>
    </div>
  );
}
